use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod annealing;
mod id;

use crate::problem::{Pose, Problem};

pub trait Solver: Sync {
    fn solve_gen<'a>(
        &self,
        problem: &'a Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>>;

    fn solve(&self, problem: &Problem) -> Pose {
        self.solve_gen(
            problem,
            Rc::new(RefCell::new(Pose {
                vertices: problem.figure.vertices.clone(),
            })),
        )
        .last()
        .unwrap()
        .take()
    }
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
