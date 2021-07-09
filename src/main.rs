use clap::App;
use clap::Arg;
use problem::Pose;
use problem::Problem;
use render::interact;

#[macro_use]
extern crate lazy_static;

mod common;
mod portal;
mod problem;
mod render;
mod solver;

use crate::common::*;

fn main() -> Result<()> {
    let app = App::new("icfpc2021")
        // Run one solver on one problem
        .subcommand(
            App::new("run")
                .arg("<SOLVER> solver name")
                .arg("<INPUT> path/to/N.problem")
                .arg("<OUTPUT> path/to/N.solution"),
        )
        // Run one solver on all problems
        .subcommand(
            App::new("solve")
                .arg("<SOLVER> solver name")
                .arg("<INPUT> path/to/problems")
                .arg("<OUTPUT> path/to/solutions"),
        )
        // Run all solvers on all problems and replace the solutions with the best one
        .subcommand(
            App::new("solve_all")
                .arg("<INPUT> path/to/problems")
                .arg("<OUTPUT> path/to/solutions"),
        )
        .subcommand(
            App::new("render")
                .arg("<INPUT> path/to/N.problem")
                .arg(Arg::new("SOLUTION").short('s').takes_value(true)),
        )
        .subcommand(
            App::new("download")
                .arg("<ID> problem N")
                .arg("<PATH> path/to/N.problem"),
        )
        .subcommand(
            App::new("upload")
                .arg("<ID> problem N")
                .arg("<PATH> path/to/N.solution"),
        )
        .subcommand(App::new("stats").arg("<INPUT> path/to/problems"));

    let matches = app.get_matches();
    match matches.subcommand() {
        Some(("run", matches)) => {
            let problem = matches.value_of("INPUT").unwrap();
            let data = std::fs::read(&problem)?;
            let problem = Problem::from_json(&data)?;
            // TODO: remove these debug prints later
            println!("{:?}", problem);
            let name = matches.value_of("SOLVER").unwrap();
            let pose = solver::SOLVERS
                .get(name)
                .expect(&format!("Failed to find solver '{}'", name))
                .solve(&problem);
            println!("{:?}", pose);
            let json = pose.to_json()?;
            std::fs::write(matches.value_of("OUTPUT").unwrap(), json)?;
        }
        Some(("solve", matches)) => {
            let name = matches.value_of("SOLVER").unwrap();
            let solver = solver::SOLVERS
                .get(name)
                .expect(&format!("Failed to find solver '{}'", name));
            let problems_path = std::path::Path::new(matches.value_of("INPUT").unwrap());
            let solutions_path =
                std::path::Path::new(matches.value_of("OUTPUT").unwrap()).join(name);
            std::fs::create_dir_all(&solutions_path)?;
            // NOTE: we're assuming the files are named N.problem as this allows to iterate them in order
            let count = problems_path.read_dir()?.count();
            for i in 1..=count {
                let problem = Problem::from_json(&std::fs::read(
                    problems_path.join(format!("{}.problem", i)),
                )?)?;
                println!("Solving {}", i);
                let pose = solver.solve(&problem);
                println!("Done, dislikes = {}", problem.dislikes(&pose));
                std::fs::write(
                    solutions_path.join(format!("{}.solution", i)),
                    pose.to_json()?,
                )?;
            }
        }
        Some(("solve_all", matches)) => {
            let problems_path = std::path::Path::new(matches.value_of("INPUT").unwrap());
            let solutions_base_path = std::path::Path::new(matches.value_of("OUTPUT").unwrap());
            // NOTE: we're assuming the files are named N.problem as this allows to iterate them in order
            let count = problems_path.read_dir()?.count();
            for i in 1..=count {
                let problem = Problem::from_json(&std::fs::read(
                    problems_path.join(format!("{}.problem", i)),
                )?)?;
                let solution_path = solutions_base_path.join(format!("{}.solution", i));
                let best_dislikes = if solution_path.exists() {
                    problem.dislikes(&Pose::from_json(&std::fs::read(&solution_path)?)?)
                } else {
                    u64::MAX
                };
                println!("Solving {}", i);
                for name in solver::SOLVERS.keys() {
                    let solutions_path = solutions_base_path.join(name);
                    std::fs::create_dir_all(&solutions_path)?;
                    let solver = solver::SOLVERS.get(name).unwrap();
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
                        }
                    }
                }
            }
        }
        Some(("render", matches)) => {
            let problem = Problem::from_json(&std::fs::read(matches.value_of("INPUT").unwrap())?)?;
            let pose = match matches.value_of("SOLUTION") {
                Some(p) => {
                    Pose::from_json(&std::fs::read(p).expect("Failed to read solution file"))
                        .expect("Failed to parse solution file")
                }
                None => Pose {
                    vertices: problem.figure.vertices.clone(),
                },
            };
            interact(problem, pose)?;
        }
        Some(("download", matches)) => {
            portal::SESSION.download_problem(
                matches.value_of("ID").unwrap().parse()?,
                matches.value_of("PATH").unwrap(),
            )?;
        }
        Some(("upload", matches)) => {
            let solution = matches.value_of("PATH").unwrap();
            let solution_data = std::fs::read(&solution)?;
            let pose = Pose::from_json(&solution_data)?;

            let id = matches.value_of("ID").unwrap();
            let problem = format!("problems/{}.problem", matches.value_of("ID").unwrap());
            let problem_data = std::fs::read(&problem)?;
            let problem = Problem::from_json(&problem_data)?;
            assert!(problem.validate(&pose), "Pose should fit into the hole");

            portal::SESSION.upload_solution(id.parse()?, solution)?;
        }
        Some(("stats", matches)) => {
            let problems_path = std::path::Path::new(matches.value_of("INPUT").unwrap());
            // NOTE: we're assuming the files are named N.problem as this allows to iterate them in order
            let count = problems_path.read_dir()?.count();
            for i in 1..=count {
                let problem = Problem::from_json(&std::fs::read(
                    problems_path.join(format!("{}.problem", i)),
                )?)?;
                println!("Problem {}: ", i);
                println!("  Hole: {} vertices", problem.hole.len());
                println!(
                    "  Figure: {} vertices, {} edges, e={}",
                    problem.figure.vertices.len(),
                    problem.figure.edges.len(),
                    problem.figure.epsilon
                );
            }
        }
        _ => (),
    }
    Ok(())
}
