use std::fmt::{self, Display, Formatter};
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

#[derive(Clone, Copy, Debug, Default)]
pub struct ViolationSummary {
    dislikes: u64,
    vertex_violation: f64,
    edge_violation: f64,
}

impl Display for ViolationSummary {
    // `f` is a buffer, and this method must write the formatted string into it
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // `write!` is like `format!`, but it will write the formatted string
        // into a buffer (the first argument)
        write!(
            f,
            "(d: {}, vertex_v: {:.3}, edge_v: {:.3}, energy: {:.3})",
            self.dislikes,
            self.vertex_violation,
            self.edge_violation,
            self.energy()
        )
    }
}

pub struct ViolationState {
    summary: ViolationSummary,
    vertex_violations: Vec<f64>,
    edge_violations: Vec<f64>,
}

impl ViolationSummary {
    fn energy(&self) -> f64 {
        self.dislikes as f64 + 100.0 * self.vertex_violation + 100.0 * self.edge_violation
    }
}

impl Solver for AnnealingSolver {
    fn solve_gen<'a>(
        &self,
        problem: &'a Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            // Show initial state to the visualizer.
            s.yield_(pose.clone());

            let mut rng: StdRng = rand::SeedableRng::seed_from_u64(42);

            // Compute how much we violate the state with current pose.
            let mut cur_violation_state = compute_violation_state(&pose.borrow(), &problem);

            // Remember the parameters of the best pose found so far.
            let mut best_violation_summary = cur_violation_state.summary.clone();
            let mut best_pose = pose.clone();

            let mut temperature = START_T;

            println!(
                "temp: {:.5}, cur_summary: {}",
                temperature, cur_violation_state.summary,
            );
            while temperature > END_T {
                let step_size = 1;
                for inner_it in 0..INNER_IT {
                    // Compute change to pose.
                    // let vertex_index: usize =
                    //     rng.gen::<usize>() % pose.borrow().vertices.len();
                    let vertex_index: usize = inner_it % pose.borrow().vertices.len();
                    let direction: usize = rng.gen::<usize>() % 4;
                    let prev_pos = pose.borrow().vertices[vertex_index];
                    let new_pos = Point {
                        x: prev_pos.x + step_size * DX[direction],
                        y: prev_pos.y + step_size * DY[direction],
                    };

                    // Compute score here.
                    // TODO: Migrate to delta-recompute here.
                    let dislikes = problem.dislikes(&pose.borrow());
                    // Vertex violation.
                    let vertex_distance = problem.min_distance_to(new_pos);
                    let delta_distance =
                        vertex_distance - cur_violation_state.vertex_violations[vertex_index];
                    // Edge deformation violation.
                    let prev_edge_violation =
                        edges_violation_after_move(vertex_index, prev_pos, &pose, &problem.figure);
                    let new_edge_violation =
                        edges_violation_after_move(vertex_index, new_pos, &pose, &problem.figure);
                    let delta_violation = new_edge_violation - prev_edge_violation;

                    let new_violation_summary = ViolationSummary {
                        dislikes,
                        vertex_violation: cur_violation_state.summary.vertex_violation
                            + delta_distance,
                        edge_violation: cur_violation_state.summary.edge_violation
                            + delta_violation,
                    };

                    pose.borrow_mut().vertices[vertex_index] = new_pos;

                    let cur_energy = cur_violation_state.summary.energy();
                    let new_energy = new_violation_summary.energy();

                    if accept_energy(cur_energy, new_energy, temperature, &mut rng) {
                        // Move if works.
                        // Compare it with best score.
                        if new_energy < best_violation_summary.energy() {
                            best_violation_summary = new_violation_summary.clone();
                            best_pose = pose.clone();
                            s.yield_(pose.clone());
                            println!("Better pose: {}", best_violation_summary);
                        }
                        cur_violation_state.vertex_violations[vertex_index] = vertex_distance;
                        // TODO: Add edge violation recalc.
                        cur_violation_state.summary = new_violation_summary.clone();
                    } else {
                        pose.borrow_mut().vertices[vertex_index] = prev_pos;
                    }
                }
                println!(
                    "temp: {:.5}, cur_summary: {}, best_summary: {}",
                    temperature, cur_violation_state.summary, best_violation_summary,
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
        total_violation +=
            edge_violation_after_move(new_position, *edge_index, *dst, &pose.borrow(), figure);
    }
    return total_violation;
}

fn edge_violation_after_move(
    new_position: Point,
    edge_index: usize,
    dst: usize,
    pose: &Pose,
    figure: &Figure,
) -> f64 {
    let new_distance = Figure::distance_squared(new_position, pose.vertices[dst]);
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

// Evaluates the current state of the pose.
fn compute_violation_state(pose: &Pose, problem: &Problem) -> ViolationState {
    // Dislikes.
    let cur_dislikes = problem.dislikes(&pose);

    // Vertex violation - how fast all vertices are from the internals of the hole.
    let mut total_vertex_distance = 0.0;
    let mut vertex_distances = vec![0.0; pose.vertices.len()];
    for (i, vertex) in pose.vertices.iter().enumerate() {
        vertex_distances[i] = problem.min_distance_to(*vertex);
        total_vertex_distance += vertex_distances[i];
    }

    // Edge deformation violation - how much are we violating deformation constraints.
    let mut edge_violation = vec![0.0; problem.figure.edges.len()];
    let mut total_edge_violation = 0.0;
    for (i, edge) in problem.figure.edges.iter().enumerate() {
        edge_violation[i] =
            edge_violation_after_move(pose.vertices[edge.v0], i, edge.v1, &pose, &problem.figure);
        total_edge_violation += edge_violation[i]
    }

    return ViolationState {
        summary: ViolationSummary {
            dislikes: cur_dislikes,
            vertex_violation: total_vertex_distance,
            edge_violation: total_edge_violation,
        },
        vertex_violations: vertex_distances,
        edge_violations: edge_violation,
    };
}
