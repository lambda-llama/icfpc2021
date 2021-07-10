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
    Ok(for i in 1..=count {
        let problem = Problem::from_json(&std::fs::read(
            problems_path.join(format!("{}.problem", i)),
        )?)?;
        let solution_path = solutions_base_path.join(format!("{}.solution", i));
        let mut best_dislikes = if solution_path.exists() {
            problem.dislikes(&Pose::from_json(&std::fs::read(&solution_path)?)?)
        } else {
            u64::MAX
        };
        println!("Solving {}", i);
        for &name in &solver_names {
            let solutions_path = solutions_base_path.join(name);
            std::fs::create_dir_all(&solutions_path)?;
            let solver = SOLVERS.get(name).unwrap();
            let pose = solver.solve(&problem);
            let dislikes = problem.dislikes(&pose);
            let valid = problem.validate(&pose);
            println!("  {}: dislikes = {}, valid = {}", name, dislikes, valid);
            if valid {
                let current_solution_path = solutions_path.join(format!("{}.solution", i));
                std::fs::write(&current_solution_path, pose.to_json()?)?;
                if best_dislikes > dislikes {
                    println!(
                        "Replacing the current best solution ({} > {})",
                        best_dislikes, dislikes
                    );
                    std::fs::copy(current_solution_path, &solution_path)?;
                    best_dislikes = dislikes;
                }
            }
        }
    })
}
