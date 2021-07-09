use std::collections::HashMap;

use crate::problem::{Pose, Problem};

pub trait Solver: Sync {
    fn solve(&self, problem: &Problem) -> Pose;
}

lazy_static! {
    pub static ref SOLVERS: HashMap<String, Box<dyn Solver>> = {
        let map = HashMap::new();
        // TODO: add solvers here
        map
    };
}
