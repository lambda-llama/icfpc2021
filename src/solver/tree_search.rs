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

            let mut order = Vec::new();
            let mut parents = vec![(0, 0); figure_size];
            {
                let mut visited = vec![false; figure_size];
                topsort(
                    0,
                    None,
                    None,
                    &mut order,
                    &mut visited,
                    &mut parents,
                    &problem.figure.vertex_edges,
                );
            }

            let mut runner = SearchRunner {
                order,
                placed: vec![false; figure_size],
                parents,
                pose: pose.borrow().clone(),
                best_dislikes: None,
                scope: s,
            };

            // Do initial placing in coordinates.
            // TODO: Use only internal coordinates here in the future.
            let (mn, mx) = problem.bounding_box();
            {
                for x in mn.x..=mx.x {
                    for y in mn.y..=mx.y {
                        runner.pose.vertices[0] = Point { x, y };
                        if !problem.contains_point(&runner.pose.vertices[runner.order[0]]) {
                            continue;
                        }
                        debug!("Placed vertex 0 in ({}, {})", x, y);
                        runner.place_vertices(1, &problem);
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
    scope: Scope<'a, (), Rc<RefCell<Pose>>>,
}

// TODO: Precalc this.
fn get_deltas(distance: i64) -> Vec<(i64, i64)> {
    let mut deltas = Vec::new();
    for dx in 0..=distance {
        if dx * dx > distance {
            break;
        }

        let dy = ((distance - dx * dx) as f64).sqrt().floor() as i64;
        if dx * dx + dy * dy == distance {
            deltas.push((dx, dy));
            if dx != 0 {
                deltas.push((-dx, dy));
            }
            if dy != 0 {
                deltas.push((dx, -dy));
            }
            if dx != 0 && dy != 0 {
                deltas.push((-dx, -dy));
            }
        }
    }
    deltas.sort();
    deltas.dedup();
    return deltas;
}

impl<'a> SearchRunner<'a> {
    fn place_vertices(&mut self, index: usize, problem: &Problem) -> Option<u64> {
        debug!("Placing vertex {}", index);
        if index == problem.figure.vertices.len() {
            let dislikes = problem.dislikes(&self.pose);

            if self.best_dislikes.unwrap_or(10000000) > dislikes {
                self.best_dislikes = Some(dislikes);
                info!("Found better placement, dislikes: {}", dislikes);
                self.scope.yield_(Rc::new(RefCell::new(self.pose.clone())));
            }
            return Some(dislikes);
        }

        let v = self.order[index];
        self.placed[v] = true;
        let (parent, parent_edge_index) = self.parents[v];
        let parent_pos = self.pose.vertices[parent];
        let mut best_result = None;

        // TODO: Precalc this.
        let parent_bounds = problem.figure.edge_len2_bounds_int(parent_edge_index);

        for parent_d in parent_bounds.0..=parent_bounds.1 {
            for (dx, dy) in get_deltas(parent_d) {
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

                if index == 0 && parent_pos.x == 56 && parent_pos.y == 4 {
                    info!(
                        "Placed {},{} in ({}, {}), delta: ({}, {})",
                        v, index, self.pose.vertices[v].x, self.pose.vertices[v].y, dx, dy
                    );
                }
                debug!(
                    "Placed {},{} in ({}, {}), delta: ({}, {})",
                    v, index, self.pose.vertices[v].x, self.pose.vertices[v].y, dx, dy
                );

                // Validate that the placement is not breaking any edges.
                // TODO: Only traverse already placed vertices.
                let mut has_violations = false;
                for &(e_id, dst) in problem.figure.vertex_edges[v].iter() {
                    if self.placed[dst] {
                        let d = Figure::distance_squared_int(
                            self.pose.vertices[v],
                            self.pose.vertices[dst],
                        );
                        let bounds = problem.figure.edge_len2_bounds_int(e_id);
                        // Broken edge, placement is invalid, returning.
                        if d < bounds.0 || d > bounds.1 {
                            debug!("Bounds violated with {}, {} out of {:?}", dst, d, bounds);
                            has_violations = true;
                            break;
                        }
                    }
                }
                if has_violations {
                    continue;
                }

                if let Some(new_dislikes) = self.place_vertices(index + 1, problem) {
                    match best_result {
                        Some(best_dislikes) => {
                            if best_dislikes > new_dislikes {
                                best_result = Some(new_dislikes);
                            }
                        }
                        None => {
                            best_result = Some(new_dislikes);
                        }
                    }
                }
            }
        }
        self.placed[v] = false;
        return best_result;
    }
}
