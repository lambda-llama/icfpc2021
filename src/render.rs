use std::cell::RefCell;
use std::rc::Rc;
use std::{thread, time};

use ordered_float::NotNan;
use raylib::prelude::*;

use crate::common::*;
use crate::problem::{Figure, Point, Pose, Problem};
use crate::solver::Solver;

struct Translator {
    x_offset: f32,
    y_offset: f32,
    zero: Point,
    max: Point,
    x_step: f32,
    y_step: f32,
}

impl Translator {
    fn new(x_offset: f32, y_offset: f32, width: f32, height: f32, p: &Problem) -> Translator {
        let (min_p, max_p) = p.bounding_box();
        let x_step = width / ((max_p.x - min_p.x) as f32);
        let y_step = height / ((max_p.y - min_p.y) as f32);
        return Translator {
            x_offset,
            y_offset,
            zero: min_p,
            max: max_p,
            x_step,
            y_step,
        };
    }

    fn translate(&self, p: &Point) -> Vector2 {
        return Vector2::new(
            ((p.x - self.zero.x) as f32) * self.x_step + self.x_offset,
            ((p.y - self.zero.y) as f32) * self.y_step + self.y_offset,
        );
    }

    fn untranslate(&self, v: &Vector2) -> Point {
        return Point {
            x: ((v.x - self.x_offset) / self.x_step + (self.zero.x as f32)).round() as i64,
            y: ((v.y - self.y_offset) / self.y_step + (self.zero.y as f32)).round() as i64,
        };
    }
}

// TODO: bool => enum (ok, close to bad, bad)
fn test_edge_length(figure: &Figure, pose: &Pose, idx: usize) -> bool {
    let (min, max) = figure.edge_len_bounds(idx);
    let len = figure.edge_len(idx, pose);
    if len < min || len > max {
        false
    } else {
        true
    }
}

fn render_problem(d: &mut RaylibDrawHandle, t: &Translator, problem: &Problem, pose: &Pose) {
    const POINT_RADIUS: f32 = 5.0;
    const LINE_THICKNESS_HOLE: f32 = 4.0;
    const LINE_THICKNESS_EDGE: f32 = 2.5;
    const COLOR_GRID: Color = Color::GRAY;
    const COLOR_HOLE: Color = Color::BLACK;
    const COLOR_VERTEX: Color = Color::DARKGREEN;
    const COLOR_EDGE_OK: Color = Color::GREEN;
    const COLOR_EDGE_BAD: Color = Color::RED;

    for x in t.zero.x..t.max.x {
        for y in t.zero.y..t.max.y {
            d.draw_pixel_v(t.translate(&Point { x, y }), COLOR_GRID);
        }
    }

    let mut last_p: Option<&Point> = problem.hole.last();
    for p in problem.hole.iter() {
        d.draw_circle_v(t.translate(&p), POINT_RADIUS, COLOR_HOLE);
        match last_p {
            Some(pp) => d.draw_line_ex(
                t.translate(&pp),
                t.translate(&p),
                LINE_THICKNESS_HOLE,
                COLOR_HOLE,
            ),
            None => {}
        }
        last_p = Some(p);
    }

    for (idx, (i, j)) in problem.figure.edges.iter().enumerate() {
        d.draw_line_ex(
            t.translate(&pose.vertices[*i as usize]),
            t.translate(&pose.vertices[*j as usize]),
            LINE_THICKNESS_EDGE,
            match test_edge_length(&problem.figure, pose, idx) {
                true => COLOR_EDGE_OK,
                false => COLOR_EDGE_BAD,
            },
        );
    }
    for p in pose.vertices.iter() {
        d.draw_circle_v(t.translate(p), POINT_RADIUS, COLOR_VERTEX);
    }
}

fn hit_test(pose: &Pose, mouse_pos: Point, dist: i64) -> Option<usize> {
    let mut points_with_dist = pose
        .vertices
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let dist = NotNan::new(Figure::distance(p, mouse_pos)).unwrap();
            (i, dist)
        })
        .collect::<Vec<_>>();
    points_with_dist.sort_unstable_by_key(|p| p.1);
    if points_with_dist[0].1.into_inner() < dist as f64 {
        Some(points_with_dist[0].0)
    } else {
        None
    }
}

pub fn interact<'a>(problem: Problem, solver: &Box<dyn Solver>, pose: Pose) -> Result<()> {
    use raylib::consts::*;

    const WINDOW_WIDTH: i32 = 1024;
    const WINDOW_HEIGHT: i32 = 768;

    const VIEWPORT_OFFSET_X: f32 = 20.0;
    const VIEWPORT_OFFSET_Y: f32 = 20.0;
    const VIEWPORT_WIDTH: f32 = 600.0;
    const VIEWPORT_HEIGHT: f32 = 600.0;

    let (mut rh, thread) = raylib::init().size(WINDOW_WIDTH, WINDOW_HEIGHT).build();

    let mut dragged_point = None;
    let t = Translator::new(
        VIEWPORT_OFFSET_X,
        VIEWPORT_OFFSET_Y,
        VIEWPORT_WIDTH,
        VIEWPORT_HEIGHT,
        &problem,
    );

    let mut gen = solver.solve_gen(&problem, Rc::new(RefCell::new(pose)));
    let pose = gen.resume().unwrap();

    while !rh.window_should_close() {
        {
            rh.set_window_title(
                &thread,
                &format!(
                    "eps: {}; dlike_score: {}; inside: {}",
                    problem.figure.epsilon,
                    problem.dislikes(&pose.borrow()),
                    problem.validate(&pose.borrow()),
                ),
            );
            let mut d = rh.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            render_problem(&mut d, &t, &problem, &pose.borrow());
        }

        if rh.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            dragged_point = hit_test(&pose.borrow(), mouse_pos, 2);
        }

        if rh.is_mouse_button_released(MouseButton::MOUSE_LEFT_BUTTON) {
            dragged_point = None;
        }

        if rh.get_gesture_detected() == GestureType::GESTURE_DRAG {
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            if let Some(idx) = dragged_point {
                pose.borrow_mut().vertices[idx] = mouse_pos;
            }
        }

        let mut need_to_sleep = true;
        if let Some(key) = rh.get_key_pressed() {
            match key {
                KeyboardKey::KEY_S => {
                    const PATH: &'static str = "./current.solution";
                    std::fs::write(PATH, pose.borrow().to_json()?)?;
                    println!("Saved to {}", PATH);
                }
                KeyboardKey::KEY_D => {
                    if gen.resume().is_some() {
                        need_to_sleep = false;
                    } else {
                        println!("WARNING: No more steps in the solver");
                    }
                }
                _ => {}
            }
        }

        if rh.is_key_down(KeyboardKey::KEY_F) {
            if gen.resume().is_some() {
                need_to_sleep = false;
            } else {
                println!("WARNING: No more steps in the solver");
            }
        }

        if need_to_sleep {
            thread::sleep(time::Duration::from_millis(5));
        }
    }
    Ok(())
}
