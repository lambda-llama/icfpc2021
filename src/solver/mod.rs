use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod annealing;
mod cons;
mod id;
mod jammer;

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
        use cons::Cons;
        let mut map: HashMap<String, Box<dyn Solver>> = HashMap::new();
        // Add solvers here
        map.insert("id".to_owned(), Box::new(id::IdSolver::default())); // Dummy solver for testing the runner.
        map.insert("annealing".to_owned(), Box::new(annealing::AnnealingSolver::default())); // Solver based on annealing.
        map.insert("jammed_annealing".to_owned(), Box::new(Cons::<jammer::JammerSolver, annealing::AnnealingSolver>::default()));
        map
    };
}
