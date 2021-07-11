use generator::Scope;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use std::{cell::RefCell, rc::Rc};

use crate::common::*;
use crate::problem::*;

use super::Solver;

#[derive(Default)]
pub struct TreeSearchSolver {
    pub timeout: Option<std::time::Duration>,
}

impl Solver for TreeSearchSolver {
    fn solve_gen<'a>(
        &self,
        mut problem: Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        let deadline = match self.timeout {
            Some(timeout) => Some(std::time::Instant::now() + timeout),
            None => None,
        };
        let mut rng: StdRng = rand::SeedableRng::seed_from_u64(42);

        generator::Gn::new_scoped_local(move |mut s| {
            s.yield_(pose.clone());

            let figure_size = problem.figure.vertices.len();
            if figure_size > 20 {
                done!();
            }

            let start_vertex = 0;
            // for i in 0..figure_size {
            //     if problem.figure.vertex_edges[i].len() > problem.figure.vertex_edges[start_vertex].len() {
            //         start_vertex = i;
            //     }
            // }

            let mut order = Vec::new();
            let mut v_in_order = vec![0; figure_size];
            let mut parents = vec![(0, 0); figure_size];
            let mut topo_vertex_edges = vec![Vec::new(); figure_size];
            {
                let mut visited = vec![false; figure_size];
                topsort(
                    start_vertex,
                    None,
                    None,
                    &mut order,
                    &mut visited,
                    &mut parents,
                    &problem.figure.vertex_edges,
                    &mut topo_vertex_edges,
                );
                for i in 0..figure_size {
                    v_in_order[order[i]] = i;
                }
            }

            let precalc_start = std::time::Instant::now();
            problem.precalc();
            let mut max_delta: usize = 0;
            for &p1 in &problem.hole {
                for &p2 in &problem.hole {
                    max_delta =
                        std::cmp::max(max_delta, Figure::distance_squared_int(p1, p2) as usize);
                }
            }
            info!("Max delta: {}", max_delta);
            let mut delta_precalc: Vec<Vec<(i64, i64)>> = vec![Vec::new(); max_delta + 1];
            let delta_sqrt = ((max_delta as f64).sqrt().ceil()) as i64 + 5;
            for dx in 0..=delta_sqrt {
                for dy in 0..=delta_sqrt {
                    let delta = (dx * dx + dy * dy) as usize;
                    if delta > max_delta {
                        break;
                    }
                    delta_precalc[delta].push((-dx, dy));
                    delta_precalc[delta].push((-dx, -dy));
                    delta_precalc[delta].push((dx, dy));
                    delta_precalc[delta].push((dx, -dy));
                }
            }
            for v in delta_precalc.iter_mut() {
                v.dedup();
                // v.shuffle(&mut rng);
            }

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

            let mut vertex_deltas: Vec<Vec<(i64, i64)>> = vec![Vec::new(); figure_size];
            for v in 0..figure_size {
                // Starting vertex has no parent, so skipping for now.
                if v == start_vertex {
                    continue;
                }

                let (_, parent_edge_index) = parents[v];
                let parent_bounds = &edge_precalc[parent_edge_index];
                for parent_d in parent_bounds.0..=parent_bounds.1 {
                    vertex_deltas[v].extend(delta_precalc[parent_d as usize].iter());
                }
                info!("Vectex {} degree: {}", v, vertex_deltas[v].len());
            }

            // Each cycle is a sequence of (destination_vertex, edge_index_leading_to_it).
            let mut vertex_cycles: Vec<Vec<Vec<(usize, usize)>>> =
                vec![vec![Vec::new(); 0]; figure_size];
            let mut path: Vec<(usize, usize)> = Vec::new();
            for v in 0..figure_size {
                find_cycles(
                    v,
                    v,
                    &problem.figure.vertex_edges,
                    &topo_vertex_edges,
                    &mut path,
                    &mut vertex_cycles[v],
                    6,
                );
                info!("Vectex {} cycles: {}", v, vertex_cycles[v].len());
            }

            for v in 0..figure_size {
                for cycle in &vertex_cycles[v] {
                    // For now only handling cycles of length 3.
                    if cycle.len() == 3 {
                        let v1 = cycle[0].0;
                        let v2 = cycle[1].0;
                        let back_edge_index = cycle[2].1;

                        // Find feasible deltas.
                        let mut delta_v1_is_feasible = vec![false; vertex_deltas[v1].len()];
                        let mut delta_v2_is_feasible = vec![false; vertex_deltas[v2].len()];
                        for (delta_v1_idx, delta_v1) in vertex_deltas[v1].iter().enumerate() {
                            for (delta_v2_idx, delta_v2) in vertex_deltas[v2].iter().enumerate() {
                                let mut deltas_are_feasible = false;
                                let delta = (delta_v1.0 + delta_v2.0, delta_v1.1 + delta_v2.1);
                                // TODO: Replace this check with a lookup to a hashtable.
                                let parent_bounds = &edge_precalc[back_edge_index];
                                for parent_d in parent_bounds.0..=parent_bounds.1 {
                                    for back_delta in delta_precalc[parent_d as usize].iter() {
                                        if delta.0 + back_delta.0 == 0
                                            && delta.1 + back_delta.1 == 0
                                        {
                                            deltas_are_feasible = true;
                                            break;
                                        }
                                    }
                                    if deltas_are_feasible {
                                        break;
                                    }
                                }

                                if deltas_are_feasible {
                                    delta_v1_is_feasible[delta_v1_idx] = true;
                                    delta_v2_is_feasible[delta_v2_idx] = true;
                                }
                            }
                        }
                        // Narrow down vertex deltas to feasible ones for v1.
                        let mut new_v1_deltas = Vec::new();
                        for (delta_v1_idx, delta_v1) in vertex_deltas[v1].iter().enumerate() {
                            if delta_v1_is_feasible[delta_v1_idx] {
                                new_v1_deltas.push(*delta_v1);
                            }
                        }
                        vertex_deltas[v1] = new_v1_deltas;
                        // Narrow down vertex deltas to feasible ones for v2.
                        let mut new_v2_deltas = Vec::new();
                        for (delta_v2_idx, delta_v2) in vertex_deltas[v2].iter().enumerate() {
                            if delta_v2_is_feasible[delta_v2_idx] {
                                new_v2_deltas.push(*delta_v2);
                            }
                        }
                        vertex_deltas[v2] = new_v2_deltas;
                    }
                }
            }

            for v in 0..figure_size {
                // Starting vertex has no parent, so skipping for now.
                if v == start_vertex {
                    continue;
                }
                info!("Vectex {} degree after pruning: {}", v, vertex_deltas[v].len());
            }

            let precalc_time_taken = std::time::Instant::now() - precalc_start;
            info!(
                "Precalc duration: {}.{}s",
                precalc_time_taken.as_secs(),
                precalc_time_taken.subsec_millis()
            );

            let mut runner = SearchRunner {
                order,
                placed: vec![false; figure_size],
                parents,
                pose: pose.borrow().clone(),
                best_dislikes: None,
                last_log_time: std::time::Instant::now(),
                iterations: 0,
                terminate: false,
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
                            &deadline,
                        );
                        if result.is_some() {
                            if result.unwrap() == 0 {
                                // TODO: optionally yield pose with optimal = Some(true)
                                done!();
                            }
                        }
                        // runner.placed[runner.order[0]] = false;
                    }
                }
            }

            // TODO: optionally yield pose with optimal = Some(true)
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
    topo_vertex_edges: &mut Vec<Vec<(usize, usize)>>,
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
        topo_vertex_edges[v].push((edge_index, dst));

        topsort(
            dst,
            Some(v),
            Some(edge_index),
            order,
            visited,
            parents,
            edges,
            topo_vertex_edges,
        );
    }
}

fn find_cycles(
    start_v: usize,
    v: usize,
    edges: &Vec<Vec<(usize, usize)>>,
    topo_vertex_edges: &Vec<Vec<(usize, usize)>>,
    path: &mut Vec<(usize, usize)>,
    cycles: &mut Vec<Vec<(usize, usize)>>,
    max_depth: usize,
) {
    if path.len() > max_depth {
        return;
    }

    for &(edge_index, dst) in &edges[v] {
        if dst == start_v {
            path.push((dst, edge_index));
            cycles.push(path.clone());
            path.pop();
        }
    }

    for &(edge_index, dst) in &topo_vertex_edges[v] {
        path.push((dst, edge_index));
        find_cycles(
            start_v,
            dst,
            edges,
            topo_vertex_edges,
            path,
            cycles,
            max_depth,
        );
        path.pop();
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
    terminate: bool,
    scope: Scope<'a, (), Rc<RefCell<Pose>>>,
}

impl<'a> SearchRunner<'a> {
    fn check_back_edges_within_hole(
        &self,
        index: usize,
        problem: &Problem,
        back_edges: &Vec<Vec<(usize, usize)>>,
    ) -> bool {
        let v = self.order[index];
        for &(_, u) in &back_edges[v] {
            if !problem.contains_segment((self.pose.vertices[u], self.pose.vertices[v])) {
                return false
            }
        }
        true
    }

    fn place_vertices(
        &mut self,
        index: usize,
        problem: &Problem,
        delta_precalc: &Vec<Vec<(i64, i64)>>,
        edge_precalc: &Vec<(i64, i64)>,
        back_edges: &Vec<Vec<(usize, usize)>>,
        deadline: &Option<std::time::Instant>,
    ) -> Option<u64> {
        if self.terminate {
            return None;
        }

        self.iterations += 1;
        if self.iterations >= 50000 {
            let log_time = std::time::Instant::now();
            let time_taken = log_time - self.last_log_time;
            info!(
                "Iterations per second: {}",
                (self.iterations as u128 * 1000) / time_taken.as_millis()
            );
            if deadline.is_some() {
                if log_time > deadline.unwrap() {
                    self.terminate = true;
                    return None;
                }
            }
            self.iterations = 0;
            self.last_log_time = log_time;
        }
        debug!("Placing vertex {}", index);
        if index == problem.figure.vertices.len() {
            // TODO: Make this incremental.
            // if !problem.contains(&self.pose) {
            //     return None;
            // }

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
                if !problem.contains_point(&self.pose.vertices[v]) ||
                   !self.check_back_edges_within_hole(index, problem, back_edges) {
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

                if let Some(new_dislikes) = self.place_vertices(
                    index + 1,
                    problem,
                    delta_precalc,
                    edge_precalc,
                    back_edges,
                    deadline,
                ) {
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
