use clap::App;
use problem::Problem;
use raylib::prelude::*;
use std::{thread, time};

#[macro_use]
extern crate lazy_static;

mod common;
mod portal;
mod problem;

use crate::common::*;

// Graphics related settings.
const WINDOW_WIDTH: i32 = 640;
const WINDOW_HEIGHT: i32 = 480;
const CELL_SIZE: i32 = 32;
const PADDING: i32 = 2;
const ROWS: usize = 5;
const COLS: usize = 5;

struct Graphics {
    rh: RaylibHandle,
    thread: RaylibThread,
}

// Responsible for solving a single test case.
struct Solver {
    map: Vec<Vec<i32>>,
    graphics: Option<Graphics>,
}

impl Solver {
    fn new(enable_graphics: bool) -> Solver {
        let graphics = match enable_graphics {
            true => {
                let (rh, thread) = raylib::init()
                    .size(WINDOW_HEIGHT, WINDOW_WIDTH)
                    .title("icfpc2021")
                    .build();
                Some(Graphics { rh, thread })
            }
            false => None,
        };

        Solver {
            map: vec![vec![1; ROWS]; COLS],
            graphics,
        }
    }

    fn run_solve_step(&mut self) {
        let r = rand::random::<usize>() % ROWS;
        let c = rand::random::<usize>() % ROWS;
        self.map[r][c] = 1 - self.map[r][c];
    }

    // Call whenever you want to show the new state of the world.
    fn draw(&mut self) {
        if self.graphics.is_none() {
            return;
        }

        // Move the graphics out to avoid borrowing `self` twice during `self.draw_impl` call.
        let mut graphics = self.graphics.take().unwrap();
        self.draw_impl(&mut graphics);
        self.graphics = Some(graphics);
    }

    fn draw_impl(&self, g: &mut Graphics) {
        let mut d = g.rh.begin_drawing(&g.thread);

        d.clear_background(Color::WHITE);
        d.draw_text(&format!("Current period: {}", 0), 12, 200, 20, Color::BLACK);

        let size = Vector2::new(CELL_SIZE as f32, CELL_SIZE as f32);
        for i in 0..COLS {
            for j in 0..ROWS {
                if self.map[i][j] == 0 {
                    continue;
                }
                let pos = Vector2::new(
                    (5 + (i as i32) * (CELL_SIZE + PADDING)) as f32,
                    (5 + (j as i32) * (CELL_SIZE + PADDING)) as f32,
                );
                d.draw_rectangle_v(pos, size, Color::BLACK);
            }
        }
    }

    fn process_events(&mut self) -> bool {
        if let Some(g) = self.graphics.as_mut() {
            if let Some(key) = g.rh.get_key_pressed() {
                match key {
                    _ => (),
                }
            }
            g.rh.window_should_close()
        } else {
            false
        }
    }
}

fn main() -> Result<()> {
    let matches = App::new("icfpc2021")
        .arg("<INPUT> path/to/N.problem")
        .subcommand(App::new("run"))
        .subcommand(App::new("render"))
        .get_matches();

    match matches.subcommand_name() {
        Some("run") => {
            let problem = matches.value_of("INPUT").unwrap();
            let data = std::fs::read(&problem)?;
            let problem = Problem::from_json(&data);
            println!("{:?}", problem);
        }
        Some("render") => {
            let mut solver = Solver::new(true);
            loop {
                solver.run_solve_step();
                solver.draw();
                if solver.process_events() {
                    return Ok(());
                }
                thread::sleep(time::Duration::from_millis(50));
            }
        }
        _ => (),
    }
    Ok(())
}
