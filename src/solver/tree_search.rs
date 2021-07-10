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

            // TODO: Use only internal coordinates here in the future.
            let bbox = problem.bounding_box();

            let mut order = Vec::new();
            let mut parents = Vec::new();
            {
                let mut visited = vec![false; figure_size];
                topsort(
                    0,
                    None,
                    &mut order,
                    &mut visited,
                    &mut parents,
                    &problem.figure.vertex_edges,
                );
            }

            let mut runner = SearchRunner {
                order,
                parents,
                pose: pose.borrow().clone(),
                scope: s,
            };

            {
                // TODO: Iterate over possible places.
                // Do initial placing in coordinates.
                runner.pose.vertices[0] = Point { x: 0, y: 0 };
                runner.place_vertices(1, &problem);
            }

            done!();
        })
    }
}

fn topsort(
    v: usize,
    p: Option<usize>,
    order: &mut Vec<usize>,
    visited: &mut Vec<bool>,
    parents: &mut Vec<usize>,
    edges: &Vec<Vec<(usize, usize)>>,
) {
    visited[v] = true;
    if p.is_some() {
        parents[v] = p.unwrap();
    }

    order.push(v);
    for &(_, dst) in &edges[v] {
        if visited[dst] {
            continue;
        }

        topsort(dst, Some(v), order, visited, parents, &edges);
    }
}

struct SearchRunner<'a> {
    // Whether vertex is already placed.
    order: Vec<usize>,
    // Parent of the vertex in topsort order.
    parents: Vec<usize>,
    pose: Pose,
    scope: Scope<'a, (), Rc<RefCell<Pose>>>,
}

// TODO: Precalc this.
fn get_deltas(distance: u64) -> Vec<(u64, u64)> {
    let mut deltas = Vec::new();
    for dx in 0..distance {
        if dx * dx > distance {
            break;
        }

        let dy = distance - dx * dx;
    }
    return deltas;
}

impl<'a> SearchRunner<'a> {
    fn place_vertices(&mut self, index: usize, problem: &Problem) -> Option<u64> {
        if index == problem.figure.vertices.len() {
            return Some(problem.dislikes(&self.pose));
        }

        let v = self.order[index];
        let parent = self.parents[v];
        let mut best_result = None;

        // TODO: Precalc this.
        let parent_d =
            Figure::distance_squared_int(self.pose.vertices[v], self.pose.vertices[parent]);

        {
            // Place vertex `v`.
            // Attach to one of the previously placed vertices.
            self.pose.vertices[index] = Point { x: 0, y: 0 };

            // TODO: Check that placement is within hole.

            // Validate that the placement is not breaking any edges.
            // TODO: Only traverse already placed vertices.
            for &(e_id, dst) in problem.figure.vertex_edges[v].iter() {
                if dst <= index {
                    let d = Figure::distance_squared_int(
                        self.pose.vertices[v],
                        self.pose.vertices[dst],
                    );
                    let bounds = problem.figure.edge_len2_bounds_int(e_id);
                    // Broken edge, placement is invalid, returning.
                    if d < bounds.0 || d > bounds.1 {
                        return None;
                    }
                }
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

            // self.scope.yield_(Rc::new(RefCell::new(self.pose.clone())));
        }
        return best_result;
    }
}
