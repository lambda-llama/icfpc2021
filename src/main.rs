use clap::App;
use clap::Arg;
use log::LevelFilter;
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
mod storage;
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
        // Run one or all solvers on one or all problems
        .subcommand(
            App::new("solve")
                .arg(
                    Arg::new("SOLVER")
                        .short('a')
                        .takes_value(true)
                        .default_missing_value(""),
                )
                .arg(Arg::new("ID").short('i').takes_value(true)),
        )
        .subcommand(
            App::new("render")
                .arg(Arg::new("ID").short('i').takes_value(true))
                .arg(Arg::new("SOLVER").short('a').takes_value(true))
                .arg(Arg::new("SOLUTION").short('s').takes_value(true)),
        )
        .subcommand(
            App::new("download")
                .arg("<ID> problem N")
                .arg("<PATH> path/to/N.problem"),
        )
        .subcommand(App::new("upload_all"))
        .subcommand(App::new("stats"));

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
            let id = matches
                .value_of("ID")
                .map(|s| s.parse().expect("Failed to parse the problem ID"));
            runner::run(solver_name, id)?;
        }
        Some(("render", matches)) => {
            let solution_path = matches
                .value_of("SOLUTION")
                .map(|p| std::path::Path::new(p));
            let id = match matches.value_of("ID") {
                Some(i) => i
                    .parse()
                    .expect(&format!("Failed to parse problem ID '{}'", i)),
                None => 1,
            };
            let solver = match matches.value_of("SOLVER") {
                Some(name) => solver::SOLVERS
                    .get(name)
                    .expect(&format!("Failed to find solver '{}'", name)),
                None => &solver::SOLVERS["id"],
            };
            interact(solution_path, solver, id)?;
        }
        Some(("download", matches)) => {
            portal::SESSION.download_problem(
                matches.value_of("ID").unwrap().parse()?,
                matches.value_of("PATH").unwrap(),
            )?;
        }
        Some(("upload_all", _matches)) => {
            for i in 1..=storage::get_problems_count() {
                let solution = storage::load_solution(i)?;
                if let Some(mut s) = solution {
                    if !s.state.valid {
                        warn!("For problem {} solution does not fit into the hole", i);
                        continue;
                    }
                    if s.state.dislikes >= s.server_state.dislikes {
                        info!(
                            "For problem {} solution with same or higher score {} was already submitted (server has {})",
                            i, s.state.dislikes, s.server_state.dislikes
                        );
                        continue;
                    }

                    warn!(
                        "Uploading solution for problem {}, dislikes: {}",
                        i, s.state.dislikes
                    );
                    portal::SESSION.upload_solution(i as u64, &s.pose)?;
                    s.server_state.dislikes = s.state.dislikes;
                    storage::save_server_state(i, &s.server_state)?;
                } else {
                    info!("No solution for problem {}", i);
                }
            }
        }
        Some(("stats", _matches)) => {
            // NOTE: we're assuming the files are named N.problem as this allows to iterate them in order
            for i in 1..=storage::get_problems_count() {
                let problem = storage::load_problem(i)?;
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
