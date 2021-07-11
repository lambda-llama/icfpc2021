use rayon::prelude::*;

use crate::problem::*;
use crate::solver::SOLVERS;
use crate::{common::*, storage};

pub fn run(solver_name: Option<&str>, id: Option<u32>) -> Result<()> {
    let mut solver_names = match solver_name {
        Some(name) => vec![name],
        None => SOLVERS.keys().map(|s| &s[..]).collect(),
    };
    solver_names.sort();
    let ids = match id {
        Some(id) => vec![id],
        None => (1..=storage::get_problems_count()).collect(),
    };
    ids.into_par_iter()
        .map(|i| -> Result<()> {
            let mut stdout = String::new();
            let problem = storage::load_problem(i)?;
            let solution = storage::load_solution(i)?;
            let mut best_dislikes = solution
                .map(|s| s.state)
                .unwrap_or(SolutionState::new())
                .dislikes;
            if best_dislikes == 0 {
                warn!("Skipping problem {} as it's been solved optimally", i);
                return Ok(());
            }
            stdout += &format!("Problem {}\n", i);
            for &name in &solver_names {
                let solver_solutions_path = storage::SOLUTIONS_PATH.join(name);
                std::fs::create_dir_all(&solver_solutions_path)?;
                let solver = SOLVERS.get(name).unwrap();
                let start = std::time::Instant::now();
                let pose = solver.solve(problem.clone());
                let time_taken = std::time::Instant::now() - start;
                let dislikes = problem.dislikes(&pose);
                let valid = problem.validate(&pose);
                stdout += &format!(
                    "  {}: dislikes = {}, valid = {}, took {}.{}s\n",
                    name,
                    dislikes,
                    valid,
                    time_taken.as_secs(),
                    time_taken.subsec_millis()
                );
                if valid {
                    let solver_solution_path =
                        solver_solutions_path.join(format!("{}.solution", i));
                    std::fs::write(&solver_solution_path, pose.to_json()?)?;
                    if best_dislikes > dislikes {
                        stdout += &format!(
                            "Replacing the current best solution ({} > {})\n",
                            best_dislikes, dislikes
                        );
                        std::fs::copy(
                            solver_solution_path,
                            &storage::SOLUTIONS_PATH.join(format!("{}.solution", i)),
                        )?;
                        best_dislikes = dislikes;
                    }
                }
            }
            print!("{}", stdout);
            Ok(())
        })
        .collect()
}
