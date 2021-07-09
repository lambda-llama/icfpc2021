use crate::problem::{Figure, Point, Pose, Problem};
use rand;

use super::Solver;

const ANNEALING_ITERATIONS: usize = 100000;
const MAX_STEP: f64 = 100.0;

const DX: [i64; 4] = [0, 1, 0, -1];
const DY: [i64; 4] = [1, 0, -1, 0];

pub struct AnnealingSolver {}

impl Solver for AnnealingSolver {
    fn solve(&self, problem: &Problem) -> Pose {
        let mut pose = Pose {
            vertices: problem.figure.vertices.clone(),
        };
        for it in 0..ANNEALING_ITERATIONS {
            let vertex_index: usize = rand::random::<usize>() % pose.vertices.len();
            let direction: usize = rand::random::<usize>() % 4;
            let step: i64 = (MAX_STEP
                * ((ANNEALING_ITERATIONS - it) as f64 / ANNEALING_ITERATIONS as f64))
                .ceil() as i64;

            let prev_pos = pose.vertices[vertex_index];
            println!("prev_pos = {:?}, step = {}", prev_pos, step);
            let new_pos = Point::new(
                prev_pos.x() + step * DX[direction],
                prev_pos.y() + step * DY[direction],
            );

            if edges_valid_after_move(vertex_index, new_pos, &pose, &problem.figure) {
                // Compute score here.
                // Compare it with best score.
                // Move if works.
                pose.vertices[vertex_index] = new_pos;
            }
        }
        return pose;
    }
}

fn edges_valid_after_move(
    vertex_index: usize,
    new_position: Point,
    pose: &Pose,
    figure: &Figure,
) -> bool {
    for (edge_index, dst) in &figure.vertex_edges[vertex_index] {
        let new_distance = Figure::distance(new_position, pose.vertices[*dst]);
        let bounds = figure.edge_len_bounds(*edge_index);
        if new_distance < bounds.0 || new_distance > bounds.1 {
            return false;
        }
    }
    return true;
}
