use std::{cell::RefCell, rc::Rc};

use crate::problem::{Pose, Problem};

use super::Solver;

#[derive(Default)]
pub struct Cons<S1: Solver + Default, S2: Solver + Default> {
    s1: S1,
    s2: S2,
}

impl<S1: Solver + Default, S2: Solver + Default> Solver for Cons<S1, S2> {
    fn solve_gen<'a>(
        &self,
        problem: &'a Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        let gen1 = self.s1.solve_gen(problem, pose.clone());
        let gen2 = self.s2.solve_gen(problem, pose);
        generator::Gn::new_scoped_local(move |mut s| {
            for pose in gen1 {
                s.yield_(pose);
            }
            for pose in gen2 {
                s.yield_(pose);
            }
            done!();
        })
    }
}
