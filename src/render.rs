use std::{thread, time};

use raylib::prelude::*;

use crate::problem::{Point, Problem};

struct Translator {
    zero: Point,
    x_step: f32,
    y_step: f32,
}

impl Translator {
    fn new(rh: &RaylibHandle, p: &Problem) -> Translator {
        let (min_p, max_p) = bounding_box(p);
        let x_step = (rh.get_screen_width() as f32) / ((max_p.x - min_p.x) as f32);
        let y_step = (rh.get_screen_height() as f32) / ((max_p.y - min_p.y) as f32);
        return Translator {
            zero: min_p,
            x_step,
            y_step,
        };
    }

    fn translate(&self, p: &Point) -> Vector2 {
        return Vector2::new(
            ((p.x - self.zero.x) as f32) * self.x_step,
            ((p.y - self.zero.y) as f32) * self.y_step,
        );
    }

    fn untranslate(&self, v: &Vector2) -> Point {
        return Point {
            x: (v.x / self.x_step + (self.zero.x as f32)).round() as i64,
            y: (v.y / self.y_step + (self.zero.y as f32)).round() as i64,
        };
    }
}

fn bounding_box(p: &Problem) -> (Point, Point) {
    let mut min_p = Point {
        x: i64::MAX,
        y: i64::MAX,
    };
    let mut max_p = Point { x: 0, y: 0 };
    let it = p.figure.vertices.iter().chain(p.hole.iter());
    for Point { x, y } in it {
        min_p.x = std::cmp::min(min_p.x, *x);
        max_p.x = std::cmp::max(max_p.x, *x);
        min_p.y = std::cmp::min(min_p.y, *y);
        max_p.y = std::cmp::max(max_p.y, *y);
    }
    return (min_p, max_p);
}

fn render_problem(d: &mut RaylibDrawHandle, t: &Translator, p: &Problem) {
    const POINT_RADIUS: f32 = 5.0;

    let mut last_p: Option<&Point> = p.hole.last();
    for p in p.hole.iter() {
        d.draw_circle_v(t.translate(&p), POINT_RADIUS, Color::BLACK);
        match last_p {
            Some(pp) => d.draw_line_v(t.translate(&pp), t.translate(&p), Color::BLACK),
            None => {}
        }
        last_p = Some(p);
    }

    let fig = &p.figure;
    for p in fig.vertices.iter() {
        d.draw_circle_v(t.translate(p), POINT_RADIUS, Color::RED);
    }
    for (i, j) in fig.edges.iter() {
        d.draw_line_v(
            t.translate(&fig.vertices[*i as usize]),
            t.translate(&fig.vertices[*j as usize]),
            Color::RED,
        );
    }
}

pub fn interact(mut p: Problem) {
    use raylib::consts::*;

    const WINDOW_WIDTH: i32 = 640;
    const WINDOW_HEIGHT: i32 = 480;
    let (mut rh, thread) = raylib::init().size(WINDOW_HEIGHT, WINDOW_WIDTH).build();

    while !rh.window_should_close() {
        let t = Translator::new(&rh, &p);
        {
            let mut d = rh.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            render_problem(&mut d, &t, &p);
        }

        if rh.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON)
            || rh.get_gesture_detected() == GestureType::GESTURE_DRAG
        {
            let kd = kd_tree::KdTree::build(p.figure.vertices.clone());
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            let targets = kd.within_radius(&mouse_pos, 2);
            if targets.len() > 0 {
                // TODO: Consider choosing the nearest target?
                let target = targets[0];
                let idx = p
                    .figure
                    .vertices
                    .iter()
                    .position(|&p| p == *target)
                    .unwrap();
                p.figure.vertices[idx] = mouse_pos;
            }
        }

        thread::sleep(time::Duration::from_millis(5));
    }
}

impl kd_tree::KdPoint for Point {
    type Scalar = i64;
    type Dim = typenum::U2;

    fn at(&self, i: usize) -> Self::Scalar {
        return if i == 0 { self.x } else { self.y };
    }
}
