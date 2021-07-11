use rayon::prelude::*;

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
            let current_solution = storage::load_solution(i)?;
            // Only filter solved solutions in "Solve all" mode.
            if id.is_none()
                && current_solution
                    .as_ref()
                    .map(|s| s.state.optimal)
                    .unwrap_or_default()
            {
                warn!("Skipping problem {} as it's been solved optimally", i);
                return Ok(());
            }
            let mut best_dislikes = current_solution
                .as_ref()
                .map(|s| s.state.dislikes)
                .unwrap_or(u64::MAX);
            stdout += &format!("Problem {}\n", i);
            for &name in &solver_names {
                let solver_solutions_path = storage::SOLUTIONS_PATH.join(name);
                std::fs::create_dir_all(&solver_solutions_path)?;
                let solver = SOLVERS.get(name).unwrap();
                let start = std::time::Instant::now();
                let solution = solver.solve(problem.clone());
                let time_taken = std::time::Instant::now() - start;
                stdout += &format!(
                    "  {}: dislikes = {}, valid = {}, took {}.{}s\n",
                    name,
                    solution.state.dislikes,
                    solution.state.valid,
                    time_taken.as_secs(),
                    time_taken.subsec_millis()
                );
                if solution.state.valid {
                    storage::save_solution(&solution, Some(name))?;
                    if best_dislikes > solution.state.dislikes {
                        stdout += &format!(
                            "Replacing the current best solution ({} > {})\n",
                            best_dislikes, solution.state.dislikes
                        );
                        storage::save_solution(&solution, None)?;
                        best_dislikes = solution.state.dislikes;
                    }
                }
            }
            print!("{}", stdout);
            Ok(())
        })
        .collect()
}
