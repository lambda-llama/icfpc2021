use generator::Scope;
use std::{cell::RefCell, rc::Rc};

use crate::common::*;
use crate::problem::*;

use super::Solver;

#[derive(Default)]
pub struct TreeSearchSolver {}

impl Solver for TreeSearchSolver {
    fn solve_gen<'a>(
        &self,
        problem: Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            s.yield_(pose.clone());

            let figure_size = problem.figure.vertices.len();

            let start_vertex = 0;
            // for i in 0..figure_size {
            //     if problem.figure.vertex_edges[i].len() > problem.figure.vertex_edges[start_vertex].len() {
            //         start_vertex = i;
            //     }
            // }

            let mut order = Vec::new();
            let mut v_in_order = vec![0; figure_size];
            let mut parents = vec![(0, 0); figure_size];
            {
                let mut visited = vec![false; figure_size];
                // TODO: Start from vertex with max degree.
                topsort(
                    start_vertex,
                    None,
                    None,
                    &mut order,
                    &mut visited,
                    &mut parents,
                    &problem.figure.vertex_edges,
                );
                for i in 0..figure_size {
                    v_in_order[order[i]] = i;
                }
            }

            let precalc_start = std::time::Instant::now();
            let mut delta_precalc: Vec<Vec<(i64, i64)>> = vec![Vec::new(); 2 * 100 * 100 + 1];
            for dx in 0..=100i64 {
                for dy in 0..=100i64 {
                    let delta = (dx * dx + dy * dy) as usize;
                    delta_precalc[delta].push((-dx, dy));
                    delta_precalc[delta].push((-dx, -dy));
                    delta_precalc[delta].push((dx, dy));
                    delta_precalc[delta].push((dx, -dy));
                }
            }
            for v in delta_precalc.iter_mut() {
                v.dedup();
            }
            let precalc_time_taken = std::time::Instant::now() - precalc_start;
            info!(
                "Precalc duration: {}.{}s",
                precalc_time_taken.as_secs(),
                precalc_time_taken.subsec_millis()
            );

            let mut edge_precalc: Vec<(i64, i64)> = Vec::new();
            for edge_index in 0..problem.figure.edges.len() {
                edge_precalc.push(problem.figure.edge_len2_bounds_int(edge_index));
            }

            let mut back_edges: Vec<Vec<(usize, usize)>> = Vec::new();
            for v in 0..figure_size {
                back_edges.push(Vec::new());
                for &(e_id, dst) in problem.figure.vertex_edges[v].iter() {
                    if v_in_order[dst] < v_in_order[v] {
                        back_edges[v].push((e_id, dst));
                    }
                }
            }

            let mut runner = SearchRunner {
                order,
                placed: vec![false; figure_size],
                parents,
                pose: pose.borrow().clone(),
                best_dislikes: None,
                last_log_time: std::time::Instant::now(),
                iterations: 0,
                scope: s,
            };

            // Do initial placing in coordinates.
            let (mn, mx) = problem.bounding_box();
            {
                for x in mn.x..=mx.x {
                    for y in mn.y..=mx.y {
                        // TODO: Can we process them in some clever order?
                        runner.pose.vertices[runner.order[0]] = Point { x, y };
                        if !problem.contains_point(&runner.pose.vertices[runner.order[0]]) {
                            continue;
                        }
                        debug!("Placed vertex {} in ({}, {})", runner.order[0], x, y);
                        // runner.placed[runner.order[0]] = true;
                        let result = runner.place_vertices(
                            1,
                            &problem,
                            &delta_precalc,
                            &edge_precalc,
                            &back_edges,
                        );
                        if result.is_some() {
                            if result.unwrap() == 0 {
                                done!();
                            }
                        }
                        // runner.placed[runner.order[0]] = false;
                    }
                }
            }

            done!();
        })
    }
}

fn topsort(
    v: usize,
    p: Option<usize>,
    p_edge_index: Option<usize>,
    order: &mut Vec<usize>,
    visited: &mut Vec<bool>,
    parents: &mut Vec<(usize, usize)>,
    edges: &Vec<Vec<(usize, usize)>>,
) {
    visited[v] = true;
    if p.is_some() {
        parents[v] = (p.unwrap(), p_edge_index.unwrap());
    }

    order.push(v);
    // TODO: Order destinations by degree.
    for &(edge_index, dst) in &edges[v] {
        if visited[dst] {
            continue;
        }

        topsort(
            dst,
            Some(v),
            Some(edge_index),
            order,
            visited,
            parents,
            &edges,
        );
    }
}

struct SearchRunner<'a> {
    // Whether vertex is already placed.
    order: Vec<usize>,
    placed: Vec<bool>,
    // Parent of the vertex in topsort order.
    parents: Vec<(usize, usize)>,
    pose: Pose,
    best_dislikes: Option<u64>,
    last_log_time: std::time::Instant,
    iterations: u64,
    scope: Scope<'a, (), Rc<RefCell<Pose>>>,
}

impl<'a> SearchRunner<'a> {
    fn place_vertices(
        &mut self,
        index: usize,
        problem: &Problem,
        delta_precalc: &Vec<Vec<(i64, i64)>>,
        edge_precalc: &Vec<(i64, i64)>,
        back_edges: &Vec<Vec<(usize, usize)>>,
    ) -> Option<u64> {
        self.iterations += 1;
        if self.iterations >= 50000 {
            let log_time = std::time::Instant::now();
            let time_taken = log_time - self.last_log_time;
            info!(
                "Iterations per second: {}",
                (self.iterations as u128 * 1000) / time_taken.as_millis()
            );
            self.iterations = 0;
            self.last_log_time = log_time;
        }
        debug!("Placing vertex {}", index);
        if index == problem.figure.vertices.len() {
            // TODO: Make this incremental.
            if !problem.contains(&self.pose) {
                return None;
            }

            let dislikes = problem.dislikes(&self.pose);

            if self.best_dislikes.unwrap_or(10000000) > dislikes {
                self.best_dislikes = Some(dislikes);
                info!("Found better placement, dislikes: {}", dislikes);
                self.scope.yield_(Rc::new(RefCell::new(self.pose.clone())));
            }
            return Some(dislikes);
        }

        let v = self.order[index];
        // self.placed[v] = true;
        let (parent, parent_edge_index) = self.parents[v];
        let parent_pos = self.pose.vertices[parent];
        let mut best_result = None;

        let parent_bounds = &edge_precalc[parent_edge_index];
        for parent_d in parent_bounds.0..=parent_bounds.1 {
            for (dx, dy) in &delta_precalc[parent_d as usize] {
                // Place vertex `v`.
                // We attach it to one of the previously placed vertices.
                self.pose.vertices[v] = Point {
                    x: parent_pos.x + dx,
                    y: parent_pos.y + dy,
                };
                if self.pose.vertices[v].x < 0 || self.pose.vertices[v].y < 0 {
                    continue;
                }
                if !problem.contains_point(&self.pose.vertices[v]) {
                    continue;
                }

                debug!(
                    "Placed {},{} in ({}, {}), delta: ({}, {})",
                    v, index, self.pose.vertices[v].x, self.pose.vertices[v].y, dx, dy
                );

                // Validate that the placement is not breaking any edges.
                let mut has_violations = false;
                for &(e_id, dst) in back_edges[v].iter() {
                    let d = Figure::distance_squared_int(
                        self.pose.vertices[v],
                        self.pose.vertices[dst],
                    );
                    let bounds = &edge_precalc[e_id];
                    // Broken edge, placement is invalid, returning.
                    if d < bounds.0 || d > bounds.1 {
                        debug!("Bounds violated with {}, {} out of {:?}", dst, d, bounds);
                        has_violations = true;
                        break;
                    }
                }
                if has_violations {
                    continue;
                }

                if let Some(new_dislikes) =
                    self.place_vertices(index + 1, problem, delta_precalc, edge_precalc, back_edges)
                {
                    if best_result.unwrap_or(1000000) > new_dislikes {
                        best_result = Some(new_dislikes);
                        if new_dislikes == 0 {
                            return best_result;
                        }
                    }
                }
            }
        }
        // self.placed[v] = false;
        return best_result;
    }
}
