use std::collections::HashMap;

mod annealing;
mod id;

use crate::problem::{Pose, Problem};

pub trait Solver: Sync {
    fn solve(&self, problem: &Problem) -> Pose;
}

lazy_static! {
    pub static ref SOLVERS: HashMap<String, Box<dyn Solver>> = {
        let mut map: HashMap<String, Box<dyn Solver>> = HashMap::new();
        // Add solvers here
        map.insert("id".to_owned(), Box::new(id::IdSolver {})); // Dummy solver for testing the runner.
        map.insert("annealing".to_owned(), Box::new(annealing::AnnealingSolver {})); // Solver based on annealing.
        map
    };
}
