use std::{thread, time};

use ordered_float::NotNan;
use raylib::prelude::*;

use crate::problem::{Figure, Point, Pose, Problem};

struct Translator {
    zero: Point,
    x_step: f32,
    y_step: f32,
}

impl Translator {
    fn new(rh: &RaylibHandle, p: &Problem) -> Translator {
        let (min_p, max_p) = bounding_box(p);
        let x_step = (rh.get_screen_width() as f32) / ((max_p.x() - min_p.x()) as f32);
        let y_step = (rh.get_screen_height() as f32) / ((max_p.y() - min_p.y()) as f32);
        return Translator {
            zero: min_p,
            x_step,
            y_step,
        };
    }

    fn translate(&self, p: &Point) -> Vector2 {
        return Vector2::new(
            ((p.x() - self.zero.x()) as f32) * self.x_step,
            ((p.y() - self.zero.y()) as f32) * self.y_step,
        );
    }

    fn untranslate(&self, v: &Vector2) -> Point {
        return Point::new(
            (v.x / self.x_step + (self.zero.x() as f32)).round() as i64,
            (v.y / self.y_step + (self.zero.y() as f32)).round() as i64,
        );
    }
}

fn bounding_box(p: &Problem) -> (Point, Point) {
    let mut min_p = Point::new(i64::MAX, i64::MAX);
    let mut max_p = Point::new(0, 0);
    let it = p.hole.iter().chain(p.hole.iter());
    for p in it {
        min_p.set_x(std::cmp::min(min_p.x(), p.x()));
        max_p.set_x(std::cmp::max(max_p.x(), p.x()));
        min_p.set_y(std::cmp::min(min_p.y(), p.y()));
        max_p.set_y(std::cmp::max(max_p.y(), p.y()));
    }
    return (min_p, max_p);
}

fn render_problem(d: &mut RaylibDrawHandle, t: &Translator, problem: &Problem, pose: &Pose) {
    const POINT_RADIUS: f32 = 5.0;

    let mut last_p: Option<&Point> = problem.hole.last();
    for p in problem.hole.iter() {
        d.draw_circle_v(t.translate(&p), POINT_RADIUS, Color::BLACK);
        match last_p {
            Some(pp) => d.draw_line_v(t.translate(&pp), t.translate(&p), Color::BLACK),
            None => {}
        }
        last_p = Some(p);
    }

    for p in pose.vertices.iter() {
        d.draw_circle_v(t.translate(p), POINT_RADIUS, Color::RED);
    }
    for (i, j) in problem.figure.edges.iter() {
        d.draw_line_v(
            t.translate(&pose.vertices[*i as usize]),
            t.translate(&pose.vertices[*j as usize]),
            Color::RED,
        );
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

pub fn interact(problem: Problem, mut pose: Pose) {
    use raylib::consts::*;

    const WINDOW_WIDTH: i32 = 640;
    const WINDOW_HEIGHT: i32 = 480;
    let (mut rh, thread) = raylib::init().size(WINDOW_HEIGHT, WINDOW_WIDTH).build();

    let mut dragged_point = None;
    while !rh.window_should_close() {
        let t = Translator::new(&rh, &problem);
        {
            let mut d = rh.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            render_problem(&mut d, &t, &problem, &pose);
        }

        if rh.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            dragged_point = hit_test(&pose, mouse_pos, 2);
        }

        if rh.is_mouse_button_released(MouseButton::MOUSE_LEFT_BUTTON) {
            dragged_point = None;
        }

        if rh.get_gesture_detected() == GestureType::GESTURE_DRAG {
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            if let Some(idx) = dragged_point {
                pose.vertices[idx] = mouse_pos;
            }
        }

        thread::sleep(time::Duration::from_millis(5));
    }
}
