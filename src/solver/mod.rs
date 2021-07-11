use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod annealing;
mod cons;
mod id;
mod jammer;
mod tree_search;
mod wave;

use crate::{problem::*, storage};

pub trait Solver: Sync {
    fn solve_gen<'a>(
        &self,
        problem: Problem,
        pose: Rc<RefCell<Pose>>,
    ) -> generator::LocalGenerator<'a, (), Rc<RefCell<Pose>>>;

    fn solve(&self, problem: Problem) -> Solution {
        let id = problem.id;
        let initial_pose = problem.figure.get_default_pose();
        let pose = self
            .solve_gen(problem.clone(), Rc::new(RefCell::new(initial_pose)))
            .last()
            .unwrap()
            .take();
        let dislikes = problem.dislikes(&pose);
        let state = SolutionState {
            dislikes,
            valid: problem.validate(&pose),
            optimal: dislikes == 0 || pose.optimal.unwrap_or_default(),
        };
        Solution {
            id,
            pose,
            state,
            server_state: storage::load_server_state(id).expect("Failed to read server state"),
        }
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
        // Discrete tree with 1 minute timeout.
        map.insert("tree_search_1min".to_owned(), Box::new(tree_search::TreeSearchSolver{
            timeout: Some(std::time::Duration::from_secs(60)),
        }));
        // Discrete tree with 10 minutes timeout.
        map.insert("tree_search_10min".to_owned(), Box::new(tree_search::TreeSearchSolver{
            timeout: Some(std::time::Duration::from_secs(10 * 60)),
        }));
        // Discrete tree with 30 minutes timeout.
        map.insert("tree_search_30min".to_owned(), Box::new(tree_search::TreeSearchSolver{
            timeout: Some(std::time::Duration::from_secs(30 * 60)),
        }));
        map
    };
}
