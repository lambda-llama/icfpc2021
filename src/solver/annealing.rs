use std::{cell::RefCell, rc::Rc};

use crate::problem::{Figure, Point, Pose, Problem};
use rand;

use super::Solver;

const OUTER_IT: usize = 10;
const INNER_IT: usize = 100000;
const MAX_STEP: i64 = 10;

const DX: [i64; 4] = [0, 1, 0, -1];
const DY: [i64; 4] = [1, 0, -1, 0];

pub struct AnnealingSolver {}

impl Solver for AnnealingSolver {
    fn solve_gen<'a>(
        &self,
        problem: &'a Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            s.yield_(pose.clone());
            let mut min_energy: u64 = 10000000;
            for outer_it in 0..OUTER_IT {
                // let step: i64 = (MAX_STEP
                //     * ((OUTER_IT - outer_it) as f64 / OUTER_IT as f64))
                //     .ceil() as i64;
                let step_size = (OUTER_IT - outer_it) as i64;
                let mut violated_edges = 0;
                for _ in 0..INNER_IT {
                    let vertex_index: usize =
                        rand::random::<usize>() % pose.borrow().vertices.len();
                    let direction: usize = rand::random::<usize>() % 4;

                    let prev_pos = pose.borrow().vertices[vertex_index];
                    let new_pos = Point::new(
                        prev_pos.x() + step_size * DX[direction],
                        prev_pos.y() + step_size * DY[direction],
                    );

                    if edges_valid_after_move(vertex_index, new_pos, &pose, &problem.figure) {
                        pose.borrow_mut().vertices[vertex_index] = new_pos;
                        s.yield_(pose.clone());
                        let dislikes = problem.dislikes(&pose.borrow());
                        let mut energy = dislikes;
                        if !problem.validate(&pose.borrow()) {
                            energy += 100000;
                        }
                        if energy < min_energy {
                            // Move if works.
                            // Compute score here.
                            // Compare it with best score.
                            min_energy = energy;
                            println!("Found better pose: {}", energy);
                        } else {
                            pose.borrow_mut().vertices[vertex_index] = prev_pos;
                            s.yield_(pose.clone());
                        }
                    } else {
                        violated_edges += 1;
                    }
                }
                println!(
                    "step_size: {}, skipped: {}/{}, energy: {}",
                    step_size, violated_edges, INNER_IT, min_energy
                );
            }
            done!()
        })
    }
}

fn edges_valid_after_move(
    vertex_index: usize,
    new_position: Point,
    pose: &Rc<RefCell<Pose>>,
    figure: &Figure,
) -> bool {
    for (edge_index, dst) in &figure.vertex_edges[vertex_index] {
        let new_distance = Figure::distance(new_position, pose.borrow().vertices[*dst]);
        let bounds = figure.edge_len_bounds(*edge_index);
        if new_distance < bounds.0 || new_distance > bounds.1 {
            return false;
        }
    }
    return true;
}
