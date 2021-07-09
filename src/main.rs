use clap::App;
use problem::Problem;
use problem::Pose;
use raylib::prelude::*;
use render::{render_problem, Translator};
use std::{thread, time};

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
        .subcommand(
            App::new("run")
                .arg("<SOLVER> solver name")
                .arg("<INPUT> path/to/N.problem")
                .arg("<OUTPUT> path/to/N.solution"),
        )
        .subcommand(
            App::new("solve")
                .arg("<SOLVER> solver name")
                .arg("<INPUT> path/to/problems")
                .arg("<OUTPUT> path/to/solutions"),
        )
        .subcommand(App::new("render").arg("<INPUT> path/to/N.problem"))
        .subcommand(
            App::new("download")
                .arg("<ID> problem N")
                .arg("<PATH> path/to/N.problem"),
        )
        .subcommand(
            App::new("upload")
                .arg("<ID> problem N")
                .arg("<PATH> path/to/N.solution"),
        );

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
        Some(("render", matches)) => {
            let problem = matches.value_of("INPUT").unwrap();
            let data = std::fs::read(&problem)?;
            let problem = Problem::from_json(&data).unwrap();

            const WINDOW_WIDTH: i32 = 640;
            const WINDOW_HEIGHT: i32 = 480;
            let (mut rh, thread) = raylib::init().size(WINDOW_HEIGHT, WINDOW_WIDTH).build();

            while !rh.window_should_close() {
                let t = Translator::new(&rh, &problem);
                let mut d = rh.begin_drawing(&thread);
                d.clear_background(Color::WHITE);
                render_problem(&mut d, &t, &problem);
                thread::sleep(time::Duration::from_millis(50));
            }
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

            portal::SESSION.upload_solution(
                id.parse()?,
                solution,
            )?;
        }
        _ => (),
    }
    Ok(())
}
