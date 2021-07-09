use crate::problem::{Pose, Problem};

use super::Solver;

pub struct IdSolver {}

impl Solver for IdSolver {
    fn solve(&self, problem: &Problem) -> Pose {
        Pose {
            vertices: problem.figure.vertices.clone(),
        }
    }
}
