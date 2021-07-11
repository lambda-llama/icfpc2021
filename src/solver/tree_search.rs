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
            if figure_size > 100 {
                done!();
            }

            let (mn, mx) = problem.bounding_box();

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

            let mut edges_consumed: Vec<i16> = vec![0; figure_size];
            let mut forward_edges: Vec<Vec<(usize, usize)>> = Vec::new();
            let mut back_edges: Vec<Vec<(usize, usize)>> = Vec::new();
            for v in 0..figure_size {
                forward_edges.push(Vec::new());
                back_edges.push(Vec::new());
                for &(e_id, dst) in problem.figure.vertex_edges[v].iter() {
                    if v_in_order[dst] < v_in_order[v] {
                        back_edges[v].push((e_id, dst));
                    } else {
                        forward_edges[v].push((e_id, dst));
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

            let lenx = mx.x - mn.x + 1;
            let leny = mx.y - mn.y + 1;
            let mut places_list: Vec<RefCell<Vec<(i64, i64)>>> =
                vec![RefCell::new(Vec::new()); figure_size];
            let mut can_place_in: Vec<Vec<Vec<i16>>> =
                vec![vec![vec![0; leny as usize]; lenx as usize]; figure_size];

            for x in mn.x..=mx.x {
                for y in mn.y..=mx.y {
                    let p = Point { x, y };
                    if !problem.contains_point(&p) {
                        continue;
                    }

                    for v in 0..figure_size {
                        can_place_in[v][(x - mn.x) as usize][(y - mn.y) as usize] += 1;
                    }

                    // Do initial placing in coordinates.
                    // TODO: Can we process them in some clever order?
                    places_list[start_vertex].borrow_mut().push((x, y));
                }
            }

            //             // Each cycle is a sequence of (destination_vertex, edge_index_leading_to_it).
            //             let mut vertex_cycles: Vec<Vec<Vec<(usize, usize)>>> =
            //                 vec![vec![Vec::new(); 0]; figure_size];
            //             let mut path: Vec<(usize, usize)> = Vec::new();
            //             for v in 0..figure_size {
            //                 find_cycles(
            //                     v,
            //                     v,
            //                     &problem.figure.vertex_edges,
            //                     &topo_vertex_edges,
            //                     &mut path,
            //                     &mut vertex_cycles[v],
            //                     6,
            //                 );
            //                 info!("Vectex {} cycles: {}", v, vertex_cycles[v].len());
            //             }
            //
            //             for v in 0..figure_size {
            //                 // Starting vertex has no parent, so skipping for now.
            //                 if v == start_vertex {
            //                     continue;
            //                 }
            //                 info!(
            //                     "Vectex {} degree after pruning: {}",
            //                     v,
            //                     vertex_deltas[v].len()
            //                 );
            //             }

            let precalc_time_taken = std::time::Instant::now() - precalc_start;
            info!(
                "Precalc duration: {}.{}s",
                precalc_time_taken.as_secs(),
                precalc_time_taken.subsec_millis()
            );

            let mut runner = SearchRunner {
                order,
                placed: vec![false; figure_size],
                delta_choice: vec![555555; figure_size],
                parents,
                pose: pose.borrow().clone(),
                best_dislikes: None,
                last_log_time: std::time::Instant::now(),
                iterations: 0,
                terminate: false,
                bbox_mn: mn,
                bbox_mx: mx,
                scope: s,
            };

            let result = runner.place_vertices(
                0,
                &problem,
                &mut places_list,
                &mut can_place_in,
                &mut edges_consumed,
                &edge_precalc,
                &back_edges,
                &forward_edges,
                &delta_precalc,
                &deadline,
            );
            if result.is_some() {
                if result.unwrap() == 0 {
                    // TODO: optionally yield pose with optimal = Some(true)
                    done!();
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
    delta_choice: Vec<usize>,
    // Parent of the vertex in topsort order.
    parents: Vec<(usize, usize)>,
    pose: Pose,
    best_dislikes: Option<u64>,
    last_log_time: std::time::Instant,
    iterations: u64,
    terminate: bool,
    bbox_mn: Point,
    bbox_mx: Point,
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
                return false;
            }
        }
        true
    }

    fn place_vertices(
        &mut self,
        index: usize,
        problem: &Problem,
        places_list: &mut Vec<RefCell<Vec<(i64, i64)>>>,
        can_place_in: &mut Vec<Vec<Vec<i16>>>,
        edges_consumed: &mut Vec<i16>,
        edge_precalc: &Vec<(i64, i64)>,
        back_edges: &Vec<Vec<(usize, usize)>>,
        forward_edges: &Vec<Vec<(usize, usize)>>,
        delta_precalc: &Vec<Vec<(i64, i64)>>,
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

        let mut best_result = None;
        // Hack around Rust rules.
        let v_places = places_list[v].take();

        for p in v_places.iter() {
            debug!("Placed vertex {} in ({}, {})", v, p.0, p.1);
            self.pose.vertices[v] = Point { x: p.0, y: p.1 };

            let mut can_continue_placement = true;
            for &(e_id, dst) in forward_edges[v].iter() {
                edges_consumed[dst] += 1;
                let mut fill_places_list = false;
                if edges_consumed[dst] == back_edges[dst].len() as i16 {
                    fill_places_list = true;
                    places_list[dst].borrow_mut().clear();
                }

                // Go over deltas precalcs here.
                let bounds = &edge_precalc[e_id];
                let mut valid_placements = 0;
                for d in bounds.0..=bounds.1 {
                    for delta in delta_precalc[d as usize].iter() {
                        let p_dst = (p.0 + delta.0, p.1 + delta.1);
                        if p_dst.0 < self.bbox_mn.x
                            || p_dst.0 > self.bbox_mx.x
                            || p_dst.1 < self.bbox_mn.y
                            || p_dst.1 > self.bbox_mx.y
                        {
                            continue;
                        }
                        // Propagate placement information.
                        can_place_in[dst][(p_dst.0 - self.bbox_mn.x) as usize]
                            [(p_dst.1 - self.bbox_mn.y) as usize] += 1;

                        // TODO: Compare with expected edges number.
                        if can_place_in[dst][(p_dst.0 - self.bbox_mn.x) as usize]
                            [(p_dst.1 - self.bbox_mn.y) as usize]
                            == 1 + edges_consumed[dst]
                        {
                            valid_placements += 1;
                            if fill_places_list {
                                places_list[dst].borrow_mut().push(p_dst);
                            }
                        }
                    }
                }

                if valid_placements == 0 {
                    can_continue_placement = false;
                    // TODO: Can't break early for now, need to complete incremental update.
                }
            }
            if can_continue_placement {
                // Dive deeper.
                if let Some(new_dislikes) = self.place_vertices(
                    index + 1,
                    problem,
                    places_list,
                    can_place_in,
                    edges_consumed,
                    edge_precalc,
                    back_edges,
                    forward_edges,
                    delta_precalc,
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

            for &(e_id, dst) in forward_edges[v].iter() {
                edges_consumed[dst] -= 1;
                // Go over deltas precalcs here.
                let bounds = &edge_precalc[e_id];
                for d in bounds.0..=bounds.1 {
                    for delta in delta_precalc[d as usize].iter() {
                        let p_dst = (p.0 + delta.0, p.1 + delta.1);
                        if p_dst.0 < self.bbox_mn.x
                            || p_dst.0 > self.bbox_mx.x
                            || p_dst.1 < self.bbox_mn.y
                            || p_dst.1 > self.bbox_mx.y
                        {
                            continue;
                        }
                        // Propagate placement information.
                        can_place_in[dst][(p_dst.0 - self.bbox_mn.x) as usize]
                            [(p_dst.1 - self.bbox_mn.y) as usize] -= 1;
                    }
                }
            }
        }

        places_list[v].replace(v_places);
        return best_result;

        // if !problem.contains_point(&self.pose.vertices[v])
        //     || !self.check_back_edges_within_hole(index, problem, back_edges)
        // {
        //     continue;
        // }
    }
}
