use rayon::prelude::*;

use crate::common::*;
use crate::problem::{Pose, Problem};
use crate::solver::SOLVERS;

pub fn run(
    problems_path: &std::path::Path,
    solutions_base_path: &std::path::Path,
    solver_name: Option<&str>,
) -> Result<()> {
    let mut solver_names = match solver_name {
        Some(name) => vec![name],
        None => SOLVERS.keys().map(|s| &s[..]).collect(),
    };
    solver_names.sort();
    let count = problems_path.read_dir()?.count();
    (1..=count)
        .into_par_iter()
        .map(|i| -> Result<()> {
            let mut stdout = String::new();
            let problem = Problem::from_json(&std::fs::read(
                problems_path.join(format!("{}.problem", i)),
            )?)?;
            let solution_path = solutions_base_path.join(format!("{}.solution", i));
            let mut best_dislikes = if solution_path.exists() {
                problem.dislikes(&Pose::from_json(&std::fs::read(&solution_path)?)?)
            } else {
                u64::MAX
            };
            stdout += &format!("Problem {}\n", i);
            for &name in &solver_names {
                let solutions_path = solutions_base_path.join(name);
                std::fs::create_dir_all(&solutions_path)?;
                let solver = SOLVERS.get(name).unwrap();
                let start = std::time::Instant::now();
                let pose = solver.solve(&problem);
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
                    let current_solution_path = solutions_path.join(format!("{}.solution", i));
                    std::fs::write(&current_solution_path, pose.to_json()?)?;
                    if best_dislikes > dislikes {
                        stdout += &format!(
                            "Replacing the current best solution ({} > {})\n",
                            best_dislikes, dislikes
                        );
                        std::fs::copy(current_solution_path, &solution_path)?;
                        best_dislikes = dislikes;
                    }
                }
            }
            print!("{}", stdout);
            Ok(())
        })
        .collect()
}
