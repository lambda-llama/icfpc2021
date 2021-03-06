use geo::algorithm::closest_point::ClosestPoint;
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use std::fmt::{self, Display, Formatter};
use std::{cell::RefCell, rc::Rc};

use crate::common::*;
use crate::problem::{Figure, Point, Pose, Problem};
use rand::rngs::StdRng;
use rand::Rng;

use super::Solver;

const INNER_IT: usize = 10000;
const START_T: f64 = 20.0;
const END_T: f64 = 5.0;
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
    deform_violation: f64,
    intersect_violations: f64,
}

impl Display for ViolationSummary {
    // `f` is a buffer, and this method must write the formatted string into it
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // `write!` is like `format!`, but it will write the formatted string
        // into a buffer (the first argument)
        write!(
            f,
            "(d: {}, vertex_v: {:.3}, deform: {:.3}, intersect: {:.3}, energy: {:.3})",
            self.dislikes,
            self.vertex_violation,
            self.deform_violation,
            self.intersect_violations,
            self.energy()
        )
    }
}

pub struct ViolationState {
    summary: ViolationSummary,
    vertex_violations: Vec<f64>,
    #[allow(dead_code)]
    deform_violations: Vec<f64>,
    intersect_violations: Vec<f64>,
}

impl ViolationSummary {
    fn energy(&self) -> f64 {
        self.dislikes as f64
            + 100.0 * self.vertex_violation
            + 100.0 * self.deform_violation
            + 1000.0 * self.intersect_violations
    }
}

impl Solver for AnnealingSolver {
    fn solve_gen<'a>(
        &self,
        problem: Problem,
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
            let mut best_pose = Rc::new((*pose).clone());

            let mut temperature = START_T;

            info!(
                "temp: {:.5}, cur_summary: {}",
                temperature, cur_violation_state.summary,
            );

            let weights = [
                1,    // Global move.
                1000, // Local move.
            ];
            let dist = WeightedIndex::new(&weights).unwrap();

            while temperature > END_T {
                let step_size = 1;
                for inner_it in 0..INNER_IT {
                    // Choose a random change to pos.
                    let action = dist.sample(&mut rng);
                    if action == 0 {
                        // Global move.
                        let choice = rng.gen::<usize>() % 4;
                        let new_pose = (*pose).clone();
                        for mut v in &mut new_pose.borrow_mut().vertices {
                            v.x += DX[choice];
                            v.y += DY[choice];
                        }

                        let new_violation_state =
                            compute_violation_state(&new_pose.borrow(), &problem);

                        let cur_energy = cur_violation_state.summary.energy();
                        let new_energy = new_violation_state.summary.energy();

                        if accept_energy(cur_energy, new_energy, temperature, &mut rng) {
                            pose.replace(new_pose.borrow().clone());
                            cur_violation_state = new_violation_state;

                            // Compare it with best score.
                            if new_energy < best_violation_summary.energy() {
                                best_violation_summary = cur_violation_state.summary.clone();
                                best_pose = Rc::new((*pose).clone());
                                s.yield_(best_pose.clone());
                                info!("[G] Better pose: {}", best_violation_summary);
                            }
                        }
                    } else if action == 1 {
                        // Local move.
                        let vertex_index: usize = inner_it % pose.borrow().vertices.len();
                        let cur_pos = pose.borrow().vertices[vertex_index];

                        let mut options = 4;
                        // We can do a mirror rotation in this case.
                        if problem.figure.vertex_edges[vertex_index].len() <= 2 {
                            options += 1;
                        }

                        let new_pos;
                        let choice = rng.gen::<usize>() % options;
                        if choice < 4 {
                            let direction: usize = choice;
                            new_pos = Point {
                                x: cur_pos.x + step_size * DX[direction],
                                y: cur_pos.y + step_size * DY[direction],
                            };
                        } else {
                            if problem.figure.vertex_edges[vertex_index].len() == 1 {
                                let dst_vertex_index =
                                    problem.figure.vertex_edges[vertex_index][0].1;
                                let dst_pos = pose.borrow().vertices[dst_vertex_index];
                                new_pos = Point {
                                    x: dst_pos.x + (dst_pos.x - cur_pos.x),
                                    y: dst_pos.y + (dst_pos.y - cur_pos.y),
                                }
                            } else {
                                let first_dst_vertex_index =
                                    problem.figure.vertex_edges[vertex_index][0].1;
                                let second_dst_vertex_index =
                                    problem.figure.vertex_edges[vertex_index][1].1;
                                let first_dst_pos =
                                    pose.borrow().vertices[first_dst_vertex_index].convert();
                                let second_dst_pos =
                                    pose.borrow().vertices[second_dst_vertex_index].convert();
                                let closest = geo::Line::new(first_dst_pos, second_dst_pos)
                                    .closest_point(&cur_pos.convert());
                                if let geo::Closest::SinglePoint(p) = closest {
                                    new_pos = Point {
                                        x: (p.x() + (p.x() - cur_pos.x as f64)).round() as i64,
                                        y: (p.y() + (p.y() - cur_pos.y as f64)).round() as i64,
                                    }
                                } else {
                                    // This is the case when vertex is on the line between two
                                    // neighbors. No-op.
                                    continue;
                                }
                            }
                        }

                        // Compute dislikes.
                        pose.borrow_mut().vertices[vertex_index] = new_pos;
                        let dislikes = problem.dislikes(&pose.borrow());
                        pose.borrow_mut().vertices[vertex_index] = cur_pos;
                        // Vertex violation.
                        let vertex_violation = problem.min_distance_to(new_pos);
                        let delta_vertex_violation =
                            vertex_violation - cur_violation_state.vertex_violations[vertex_index];
                        // Edge deformation violation.
                        // TODO: Take the previous violation from map.
                        let cur_deform_violation = vertex_edges_deform_violation(
                            vertex_index,
                            cur_pos,
                            &pose,
                            &problem.figure,
                        );
                        let new_deform_violation = vertex_edges_deform_violation(
                            vertex_index,
                            new_pos,
                            &pose,
                            &problem.figure,
                        );
                        let delta_deform_violation = new_deform_violation - cur_deform_violation;

                        // Edge intersection violation.
                        let mut new_edge_intersect_violations = Vec::new();
                        let mut delta_intersect_violation = 0.0;
                        for (edge_index, dst) in &problem.figure.vertex_edges[vertex_index] {
                            let new_edge_intersect_violation =
                                problem.edge_intersections(new_pos, pose.borrow().vertices[*dst]);
                            new_edge_intersect_violations
                                .push((*edge_index, new_edge_intersect_violation));
                            delta_intersect_violation += new_edge_intersect_violation
                                - cur_violation_state.intersect_violations[*edge_index];
                        }

                        let new_violation_summary = ViolationSummary {
                            dislikes,
                            vertex_violation: cur_violation_state.summary.vertex_violation
                                + delta_vertex_violation,
                            deform_violation: cur_violation_state.summary.deform_violation
                                + delta_deform_violation,
                            intersect_violations: cur_violation_state.summary.intersect_violations
                                + delta_intersect_violation,
                        };

                        let cur_energy = cur_violation_state.summary.energy();
                        let new_energy = new_violation_summary.energy();

                        if accept_energy(cur_energy, new_energy, temperature, &mut rng) {
                            // Do a change.
                            pose.borrow_mut().vertices[vertex_index] = new_pos;
                            cur_violation_state.vertex_violations[vertex_index] = vertex_violation;
                            for (edge_index, intersect_violation) in &new_edge_intersect_violations
                            {
                                cur_violation_state.intersect_violations[*edge_index] =
                                    *intersect_violation;
                            }
                            // TODO: Add edge violation recalc.
                            cur_violation_state.summary = new_violation_summary.clone();

                            // Compare it with best score.
                            if new_energy < best_violation_summary.energy() {
                                best_violation_summary = new_violation_summary.clone();
                                best_pose = Rc::new((*pose).clone());
                                s.yield_(best_pose.clone());
                                info!("[L] Better pose: {}", best_violation_summary);
                            }
                        }
                    } else {
                        panic!("Illegal action {}", action);
                    }
                }
                info!(
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

fn vertex_edges_deform_violation(
    vertex_index: usize,
    vertex_position: Point,
    pose: &Rc<RefCell<Pose>>,
    figure: &Figure,
) -> f64 {
    let mut total_violation = 0.0;
    for (edge_index, dst) in &figure.vertex_edges[vertex_index] {
        total_violation += edge_deform_violation(
            *edge_index,
            vertex_position,
            pose.borrow().vertices[*dst],
            figure,
        );
    }
    return total_violation;
}

fn edge_deform_violation(
    edge_index: usize,
    src_pos: Point,
    dst_pos: Point,
    figure: &Figure,
) -> f64 {
    let new_distance = Figure::distance_squared(src_pos, dst_pos);
    let bounds = figure.edge_len2_bounds(edge_index);
    if new_distance < bounds.0 {
        return bounds.0 - new_distance;
    } else if new_distance > bounds.1 {
        return new_distance - bounds.1;
    }
    return 0.0;
}

#[allow(dead_code)]
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
    // Dislikes and
    // Vertex violation - how fast all vertices are from the internals of the hole.
    let mut total_vertex_violation = 0.0;
    let mut vertex_violations = vec![0.0; pose.vertices.len()];
    for (v_index, vertex) in pose.vertices.iter().enumerate() {
        vertex_violations[v_index] = problem.min_distance_to(*vertex);
        total_vertex_violation += vertex_violations[v_index];
    }

    // Edge deformation violation - how much are we violating deformation constraints.
    let mut deform_violations = vec![0.0; problem.figure.edges.len()];
    let mut total_deform_violation = 0.0;
    let mut intersect_violations = vec![0.0; problem.figure.edges.len()];
    let mut total_intersect_violation = 0.0;
    for (e_index, edge) in problem.figure.edges.iter().enumerate() {
        deform_violations[e_index] = edge_deform_violation(
            e_index,
            pose.vertices[edge.v0],
            pose.vertices[edge.v1],
            &problem.figure,
        );
        total_deform_violation += deform_violations[e_index];

        intersect_violations[e_index] =
            problem.edge_intersections(pose.vertices[edge.v0], pose.vertices[edge.v1]);
        total_intersect_violation += intersect_violations[e_index];
    }

    return ViolationState {
        summary: ViolationSummary {
            dislikes: problem.dislikes(&pose),
            vertex_violation: total_vertex_violation,
            deform_violation: total_deform_violation,
            intersect_violations: total_intersect_violation,
        },
        vertex_violations,
        deform_violations,
        intersect_violations,
    };
}
