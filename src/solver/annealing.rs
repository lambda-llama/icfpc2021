use std::{cell::RefCell, rc::Rc};

use crate::problem::{Figure, Point, Pose, Problem};
use rand::rngs::StdRng;
use rand::Rng;

use super::Solver;

const INNER_IT: usize = 100000;
const START_T: f64 = 10.0;
const END_T: f64 = 0.1;
const T_DECAY: f64 = 0.97;
// const MAX_STEP: i64 = 10;

const DX: [i64; 4] = [0, 1, 0, -1];
const DY: [i64; 4] = [1, 0, -1, 0];

#[derive(Default)]
pub struct AnnealingSolver {}

impl Solver for AnnealingSolver {
    fn solve_gen<'a>(
        &self,
        problem: &'a Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            let mut rng: StdRng = rand::SeedableRng::seed_from_u64(42);
            s.yield_(pose.clone());
            let mut best_pose = pose.clone();
            let mut total_vertex_distance = 0.0;
            let mut best_dislikes = problem.dislikes(&pose.borrow());
            let mut cur_dislikes = problem.dislikes(&pose.borrow());
            let mut vertex_distances = vec![0.0; pose.borrow().vertices.len()];
            for (i, vertex) in pose.borrow().vertices.iter().enumerate() {
                vertex_distances[i] = problem.min_distance_to(*vertex);
                total_vertex_distance += vertex_distances[i];
            }
            let mut edge_violation = vec![0.0; problem.figure.edges.len()];
            let mut total_edge_violation = 0.0;
            for (i, edge) in problem.figure.edges.iter().enumerate() {
                edge_violation[i] = edge_violation_after_move(
                    pose.borrow().vertices[edge.0],
                    i,
                    edge.1,
                    &pose,
                    &problem.figure,
                );
                total_edge_violation += edge_violation[i]
            }
            let mut cur_energy = problem.dislikes(&pose.borrow()) as f64
                + 100.0 * total_vertex_distance
                + 100.0 * total_edge_violation;
            let mut min_energy: f64 = cur_energy;

            println!(
                "temperature: {}, edge_violation: {}, energy: {}, total_vertex_distance: {}, dislikes: {}, min_energy: {}, best_dislikes: {}",
                START_T, total_edge_violation, cur_energy, total_vertex_distance, cur_dislikes, min_energy, best_dislikes
            );

            let mut temperature = START_T;
            while temperature > END_T {
                let step_size = 1;
                for inner_it in 0..INNER_IT {
                    // let vertex_index: usize =
                    //     rand::random::<usize>() % pose.borrow().vertices.len();
                    let vertex_index: usize = inner_it % pose.borrow().vertices.len();
                    let direction: usize = rand::random::<usize>() % 4;
                    let prev_pos = pose.borrow().vertices[vertex_index];
                    let new_pos = Point {
                        x: prev_pos.x + step_size * DX[direction],
                        y: prev_pos.y + step_size * DY[direction],
                    };
                    // Compute score here.
                    let vertex_distance = problem.min_distance_to(new_pos);
                    let delta_distance = vertex_distance - vertex_distances[vertex_index];

                    let prev_edge_violation =
                        edges_violation_after_move(vertex_index, prev_pos, &pose, &problem.figure);
                    let new_edge_violation =
                        edges_violation_after_move(vertex_index, new_pos, &pose, &problem.figure);
                    assert!(new_edge_violation >= 0.0);
                    let delta_violation = new_edge_violation - prev_edge_violation;

                    pose.borrow_mut().vertices[vertex_index] = new_pos;
                    let dislikes = problem.dislikes(&pose.borrow());
                    let energy = dislikes as f64
                        + 100.0 * (total_vertex_distance + delta_distance)
                        + 100.0 * (total_edge_violation + delta_violation);
                    if accept_energy(cur_energy, energy, temperature, &mut rng) {
                        // Move if works.
                        // Compare it with best score.
                        if energy < min_energy {
                            min_energy = energy;
                            best_dislikes = dislikes;
                            best_pose = pose.clone();
                            s.yield_(pose.clone());
                            println!("Found better pose: {}", energy);
                        }
                        vertex_distances[vertex_index] = vertex_distance;
                        // edge_violation[vertex_index] = new_edge_violation;
                        total_vertex_distance += delta_distance;
                        total_edge_violation += delta_violation;
                        cur_dislikes = dislikes;
                        cur_energy = energy;
                    } else {
                        pose.borrow_mut().vertices[vertex_index] = prev_pos;
                    }
                }
                println!(
                    "temperature: {}, edge_violation: {}, energy: {}, total_vertex_distance: {}, dislikes: {}, min_energy: {}, best_dislikes: {}",
                    temperature, total_edge_violation, cur_energy, total_vertex_distance, cur_dislikes, min_energy, best_dislikes
                );
                // s.yield_(pose.clone());
                temperature *= T_DECAY;
            }
            s.yield_(best_pose.clone());
            done!()
        })
    }
}

fn accept_energy(prev_energy: f64, new_energy: f64, temperature: f64, rng: &mut StdRng) -> bool {
    return ((prev_energy - new_energy) / temperature).exp() > rng.gen();
}

fn edges_violation_after_move(
    vertex_index: usize,
    new_position: Point,
    pose: &Rc<RefCell<Pose>>,
    figure: &Figure,
) -> f64 {
    let mut total_violation = 0.0;
    for (edge_index, dst) in &figure.vertex_edges[vertex_index] {
        total_violation += edge_violation_after_move(new_position, *edge_index, *dst, pose, figure);
    }
    return total_violation;
}

fn edge_violation_after_move(
    new_position: Point,
    edge_index: usize,
    dst: usize,
    pose: &Rc<RefCell<Pose>>,
    figure: &Figure,
) -> f64 {
    let new_distance = Figure::distance_squared(new_position, pose.borrow().vertices[dst]);
    let bounds = figure.edge_len2_bounds(edge_index);
    if new_distance < bounds.0 {
        return bounds.0 - new_distance;
    } else if new_distance > bounds.1 {
        return new_distance - bounds.1;
    }
    return 0.0;
}

fn edges_valid_after_move(
    vertex_index: usize,
    new_position: Point,
    pose: &Rc<RefCell<Pose>>,
    figure: &Figure,
) -> bool {
    for (edge_index, dst) in &figure.vertex_edges[vertex_index] {
        let new_distance = Figure::distance_squared(new_position, pose.borrow().vertices[*dst]);
        let bounds = figure.edge_len2_bounds(*edge_index);
        if new_distance < bounds.0 || new_distance > bounds.1 {
            return false;
        }
    }
    return true;
}
