use std::{cell::RefCell, rc::Rc};

use geo::relate::Relate;
use geomath::prelude::coordinates::Polar;

use crate::common::*;
use crate::problem::*;

use super::Solver;

#[derive(Default)]
pub struct WaveSolver {}

impl Solver for WaveSolver {
    fn solve_gen<'a>(
        &self,
        problem: Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            s.yield_(pose.clone());

            // Enumerate edges back and forth and try to move the vertices closer to the center
            // or back out (depending on the wave direction)
            // Invariants:
            //   - All vertices stay inside the hole
            //   - TODO: All edges stay inside the hole
            let (min_p, max_p) = problem.bounding_box();
            let center = Point {
                x: (min_p.x + (max_p.x - min_p.x) / 2),
                y: (min_p.y + (max_p.y - min_p.y) / 2),
            };

            let mut direction_to_center = true;

            for _iterations in 0..1000 {
                info!("Direction to center: {}", direction_to_center);
                if problem.validate(&pose.borrow()) {
                    break;
                }

                for idx in 0..problem.figure.edges.len() {
                    if problem.figure.test_edge_len2(idx, &pose.borrow()) != EdgeTestResult::Ok {
                        info!("Illegal edge {}", idx);

                        let e = &problem.figure.edges[idx];
                        let len0 = Figure::distance_squared(pose.borrow().vertices[e.v0], center);
                        let len1 = Figure::distance_squared(pose.borrow().vertices[e.v1], center);
                        let (dyn_idx, stat_idx) = if (len0 > len1) ^ direction_to_center {
                            (e.v1, e.v0)
                        } else {
                            (e.v0, e.v1)
                        };

                        let v_dyn = pose.borrow().vertices[dyn_idx];
                        let v_stat = pose.borrow().vertices[stat_idx];

                        let v_fig_dyn = problem.figure.vertices[dyn_idx];
                        let v_fig_stat = problem.figure.vertices[stat_idx];

                        // Move it in a way to minimize the sum of errors
                        // TODO: rotate
                        let mut vec = geomath::vector::vec2(
                            (v_dyn.x - v_stat.x) as f64,
                            (v_dyn.y - v_stat.y) as f64,
                        );
                        let base_len = vec.rho();
                        let target_len = geomath::vector::vec2(
                            (v_fig_dyn.x - v_fig_stat.x) as f64,
                            (v_fig_dyn.y - v_fig_stat.y) as f64,
                        )
                        .rho();
                        let len_diff = target_len - base_len;
                        // Offset the base length to prevent getting stuck - edges will continue moving
                        let base_len = base_len + len_diff / 10.0;
                        let v_old = pose.borrow().vertices[dyn_idx];
                        let mut best_vertex = v_old;
                        let mut best_sum = f64::MAX;
                        for _ in 0..8 {
                            vec.set_phi(vec.phi() + std::f64::consts::FRAC_PI_4);
                            let mut d = 0.0;
                            for _ in 0..10 {
                                d += len_diff / 10.0;
                                vec.set_rho(base_len + d);
                            }
                            let p =
                                geo::Point::new(vec.x + v_stat.x as f64, vec.y + v_stat.y as f64);
                            let rel = problem.poly.relate(&p);
                            if rel.is_within() || rel.is_intersects() {
                                let v = p.convert();
                                pose.borrow_mut().vertices[dyn_idx] = v;
                                let sum = sum_of_diffs(&problem, dyn_idx, &pose.borrow());
                                if sum < best_sum {
                                    best_vertex = v;
                                    best_sum = sum;
                                }
                            }
                        }
                        info!("Setting new vertex position for {}", dyn_idx);
                        pose.borrow_mut().vertices[dyn_idx] = best_vertex;
                        s.yield_(pose.clone());
                    }
                }
                direction_to_center = !direction_to_center;
            }

            s.yield_(pose);

            done!();
        })
    }
}

fn sum_of_diffs(problem: &Problem, idx: usize, pose: &Pose) -> f64 {
    let mut sum = 0.0f64;
    for (e_idx, _) in &problem.figure.vertex_edges[idx] {
        sum += problem.figure.edge_len2_diff(*e_idx, pose).abs();
    }
    sum
}
