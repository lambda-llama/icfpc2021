use std::{cell::RefCell, rc::Rc};

use crate::problem::{Pose, Problem};

use super::Solver;

#[derive(Default)]
pub struct IdSolver {}

impl Solver for IdSolver {
    fn solve_gen<'a>(
        &self,
        _problem: Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>> {
        generator::Gn::new_scoped_local(move |mut s| {
            s.yield_(pose);
            done!();
        })
    }
}
