use clap::App;
use problem::Problem;
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
            portal::SESSION.upload_solution(
                matches.value_of("ID").unwrap().parse()?,
                matches.value_of("PATH").unwrap(),
            )?;
        }
        _ => (),
    }
    Ok(())
}
