use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod annealing;
mod cons;
mod id;
mod jammer;
mod tree_search;
mod wave;

use crate::problem::{Pose, Problem};

pub trait Solver: Sync {
    fn solve_gen<'a>(
        &self,
        problem: Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>>;

    fn solve(&self, problem: Problem) -> Pose {
        let vertices = problem.figure.vertices.clone();
        self.solve_gen(
            problem,
            Rc::new(RefCell::new(Pose {
                vertices,
                bonuses: vec![],
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
        // Dummy solver for testing the runner.
        map.insert("id".to_owned(), Box::new(id::IdSolver::default()));
        // Solver based on annealing.
        map.insert("annealing".to_owned(), Box::new(annealing::AnnealingSolver::default()));
        // Jam all vertices in and try to fix the edges
        map.insert("jammed_wave".to_owned(), Box::new(Cons::<jammer::JammerSolver, wave::WaveSolver>::default()));
        // Discrete tree search.
        map.insert("tree_search".to_owned(), Box::new(tree_search::TreeSearchSolver::default()));
        map
    };
}
