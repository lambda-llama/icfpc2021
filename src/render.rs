use raylib::prelude::*;

use crate::problem::{Point, Problem};

pub struct Translator {
    zero: Point,
    x_step: f32,
    y_step: f32,
}

impl Translator {
    pub fn new(rh: &RaylibHandle, p: &Problem) -> Translator {
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

pub fn render_problem(d: &mut RaylibDrawHandle, t: &Translator, p: &Problem) {
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
