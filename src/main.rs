use clap::App;
use clap::Arg;
use log::LevelFilter;
use problem::Pose;
use problem::Problem;
use render::interact;
use simplelog::{ColorChoice, Config, TerminalMode};

#[macro_use]
extern crate generator;
#[macro_use]
extern crate lazy_static;

mod common;
mod portal;
mod problem;
mod render;
mod runner;
mod solver;
mod transform;

use crate::common::*;

fn main() -> Result<()> {
    let app = App::new("icfpc2021")
        .arg(
            Arg::new("VERBOSE")
                .short('v')
                .takes_value(false)
                .multiple_occurrences(true),
        )
        // Run one solver on one problem
        .subcommand(
            App::new("run")
                .arg("<SOLVER> solver name")
                .arg("<INPUT> path/to/N.problem")
                .arg("<OUTPUT> path/to/N.solution"),
        )
        // Run one or all solvers on all problems
        .subcommand(
            App::new("solve")
                .arg(Arg::new("INPUT").default_value("./problems"))
                .arg(Arg::new("OUTPUT").default_value("./solutions"))
                .arg(
                    Arg::new("SOLVER")
                        .short('a')
                        .takes_value(true)
                        .default_missing_value(""),
                ),
        )
        .subcommand(
            App::new("render")
                .arg("<INPUT> path/to/N.problem")
                .arg(Arg::new("SOLVER").short('a').takes_value(true))
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
        .subcommand(
            App::new("upload_all")
                .arg(Arg::new("PROBLEMS_PATH").default_value("./problems"))
                .arg(Arg::new("SOLUTIONS_PATH").default_value("./solutions")),
        )
        .subcommand(App::new("stats").arg("<INPUT> path/to/problems"));

    let app_matches = app.get_matches();

    let filter = match app_matches.occurrences_of("VERBOSE") {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        3 => LevelFilter::Trace,
        _ => panic!("No verbosity levels beyond -vvv"),
    };
    simplelog::TermLogger::init(
        filter,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?;

    match app_matches.subcommand() {
        Some(("run", matches)) => {
            let problem = matches.value_of("INPUT").unwrap();
            let data = std::fs::read(&problem)?;
            let problem = Problem::from_json(&data)?;
            // TODO: remove these debug prints later
            println!("{:?}", problem);
            let name = matches.value_of("SOLVER").unwrap();
            let start = std::time::Instant::now();
            let pose = solver::SOLVERS
                .get(name)
                .expect(&format!("Failed to find solver '{}'", name))
                .solve(&problem);
            let dislikes = problem.dislikes(&pose);
            let valid = problem.validate(&pose);
            let time_taken = std::time::Instant::now() - start;
            let json = pose.to_json()?;
            println!(
                "dislikes = {}, valid = {}, took {}.{}s",
                dislikes,
                valid,
                time_taken.as_secs(),
                time_taken.subsec_millis()
            );
            std::fs::write(matches.value_of("OUTPUT").unwrap(), json)?;
        }
        Some(("solve", matches)) => {
            let solver_name = match matches.value_of("SOLVER") {
                Some("") | None => None,
                Some(name) => {
                    solver::SOLVERS
                        .get(name)
                        .expect(&format!("Failed to find solver '{}'", name));
                    Some(name)
                }
            };
            let problems_path = std::path::Path::new(matches.value_of("INPUT").unwrap());
            let solutions_base_path = std::path::Path::new(matches.value_of("OUTPUT").unwrap());
            runner::run(problems_path, solutions_base_path, solver_name)?;
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
                    bonuses: vec![],
                },
            };
            let solver = match matches.value_of("SOLVER") {
                Some(name) => solver::SOLVERS
                    .get(name)
                    .expect(&format!("Failed to find solver '{}'", name)),
                None => &solver::SOLVERS["id"],
            };
            interact(problem, solver, pose)?;
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
        Some(("upload_all", matches)) => {
            let problems_path = std::path::Path::new(matches.value_of("PROBLEMS_PATH").unwrap());
            let solutions_path = std::path::Path::new(matches.value_of("SOLUTIONS_PATH").unwrap());
            let count = problems_path.read_dir()?.count();

            for i in 1..count {
                let problem = Problem::from_json(&std::fs::read(
                        problems_path.join(format!("{}.problem", i)),
                        )?)?;
                let solution_path = solutions_path.join(format!("{}.solution", i));
                let solution_data = std::fs::read(&solution_path)?;
                let pose = Pose::from_json(&solution_data)?;
                assert!(problem.validate(&pose), "Pose should fit into the hole");

                // portal::SESSION.upload_solution(i as u64, solution_path.to_str().unwrap())?;
            }

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
