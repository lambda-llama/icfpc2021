use std::{cell::RefCell, rc::Rc};

use geo::relate::Relate;

use crate::problem::*;

use super::Solver;

#[derive(Default)]
pub struct JammerSolver {}

impl Solver for JammerSolver {
    fn solve_gen<'a>(
        &self,
        problem: &'a Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            s.yield_(pose.clone());

            // Step 1 - Jam all outside vertices in
            let (min_p, max_p) = problem.bounding_box();
            let center = geo::Point::new(
                (min_p.x + (max_p.x - min_p.x) / 2) as f64,
                (min_p.y + (max_p.y - min_p.y) / 2) as f64,
            );
            let idx_vertices = pose
                .borrow()
                .vertices
                .iter()
                .cloned()
                .enumerate()
                .collect::<Vec<_>>();
            for (idx, v) in idx_vertices {
                let mut p = geo::Point::new(v.x as f64, v.y as f64);
                let mut rel = problem.poly.relate(&p);
                if !(rel.is_within() || rel.is_intersects()) {
                    while !(rel.is_within() || rel.is_intersects()) {
                        println!("{:?}", p);
                        if (p.x() - center.x()).abs() < (p.y() - center.y()).abs() {
                            if p.y() > center.y() {
                                p.set_y(p.y() - 1.0);
                            } else {
                                p.set_y(p.y() + 1.0);
                            }
                        } else {
                            if p.x() > center.x() {
                                p.set_x(p.x() - 1.0);
                            } else {
                                p.set_x(p.x() + 1.0);
                            }
                        }
                        rel = problem.poly.relate(&p);
                    }
                    let new_p = Point {
                        x: p.x().trunc() as i64,
                        y: p.y().trunc() as i64,
                    };
                    pose.borrow_mut().vertices[idx] = new_p;
                    s.yield_(pose.clone());
                }
            }

            done!();
        })
    }
}
