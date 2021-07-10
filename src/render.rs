use std::cell::RefCell;
use std::ffi::CString;
use std::rc::Rc;
use std::{thread, time};

use ordered_float::NotNan;
use raylib::prelude::*;

use crate::common::*;
use crate::problem::*;
use crate::solver::Solver;
use crate::transform::Transform;

#[derive(Debug)]
enum Tool {
    Move,
    Center,
}

struct GuiState {
    pub tool: Tool,
    pub dragged_point: Option<usize>,
}

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

fn render_gui(
    d: &mut RaylibDrawHandle,
    thread: &RaylibThread,
    state: &GuiState,
    problem: &Problem,
    pose: &Pose,
) {
    // Window title
    d.set_window_title(
        &thread,
        &format!(
            "eps: {}; dlike_score: {}; inside: {}",
            problem.figure.epsilon,
            problem.dislikes(pose),
            problem.validate(pose),
        ),
    );

    // Status bar
    let text = format!("Tool: {:?}", state.tool);
    d.gui_status_bar(
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: d.get_screen_width() as f32,
            height: 25.0,
        },
        Some(&CString::new(text).unwrap()),
    );

    // Help bar
    let mut text =
        b"Tools: Q - Move, W - Center\nMisc: S - Save, D - Step solver, F - Run solver".to_owned();
    d.gui_text_box_multi(
        Rectangle {
            x: 0.0,
            y: d.get_screen_height() as f32 - 34.0,
            width: d.get_screen_width() as f32,
            height: 34.0,
        },
        &mut text,
        false,
    );
}

fn render_problem(
    d: &mut RaylibDrawHandle,
    state: &GuiState,
    t: &Translator,
    problem: &Problem,
    pose: &Pose,
) {
    const POINT_RADIUS: f32 = 5.0;
    const POINT_RADIUS_BONUS_UNLOCK: f32 = 6.0;
    const POINT_RADIUS_GRID_HIGHLIGHT: f32 = 2.0;
    const POINT_RADIUS_GRID_HIGHLIGHT2: f32 = 3.0;
    const LINE_THICKNESS_HOLE: f32 = 4.0;
    const LINE_THICKNESS_EDGE: f32 = 2.5;
    const COLOR_GRID: Color = Color::GRAY;
    const COLOR_GRID_ONE_EDGE: Color = Color::ORANGE;
    const COLOR_GRID_ALL_EDGES: Color = Color::GREEN;
    const COLOR_HOLE: Color = Color::BLACK;
    const COLOR_BONUS_UNLOCK: Color = Color::GOLD;
    const COLOR_VERTEX: Color = Color::DARKGREEN;
    const COLOR_EDGE_OK: Color = Color::GREEN;
    const COLOR_EDGE_TOO_SHORT: Color = Color::BLUE;
    const COLOR_EDGE_TOO_LONG: Color = Color::RED;

    // Grid
    let connected_edge_bounds = state.dragged_point.map(|v_idx| {
        problem.figure.vertex_edges[v_idx]
            .iter()
            .map(|&(e, v)| (v, problem.figure.edge_len2_bounds(e)))
            .collect::<Vec<_>>()
    });
    for x in t.zero.x..t.max.x {
        for y in t.zero.y..t.max.y {
            let (all, any) = if let Some(connected_edge_bounds) = connected_edge_bounds.as_ref() {
                let mut all = true;
                let mut any = false;
                for &(v_idx, (min, max)) in connected_edge_bounds.iter() {
                    let v = pose.vertices[v_idx];
                    let dist = Figure::distance_squared(Point { x, y }, v);
                    if min <= dist && dist <= max {
                        any = true;
                    } else {
                        all = false;
                    }
                }
                (all, any)
            } else {
                (false, false)
            };
            let grid_point = t.translate(&Point { x, y });
            if all {
                d.draw_circle_v(
                    grid_point,
                    POINT_RADIUS_GRID_HIGHLIGHT2,
                    COLOR_GRID_ALL_EDGES,
                )
            } else if any {
                d.draw_circle_v(grid_point, POINT_RADIUS_GRID_HIGHLIGHT, COLOR_GRID_ONE_EDGE)
            } else {
                d.draw_pixel_v(grid_point, COLOR_GRID);
            };
        }
    }

    // Hole
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

    // Bonus unlocks
    for b in problem.bonuses.iter() {
        d.draw_circle_v(
            t.translate(&b.position),
            POINT_RADIUS_BONUS_UNLOCK,
            COLOR_BONUS_UNLOCK,
        );
    }

    // Edges
    for (idx, e) in problem.figure.edges.iter().enumerate() {
        d.draw_line_ex(
            t.translate(&pose.vertices[e.v0 as usize]),
            t.translate(&pose.vertices[e.v1 as usize]),
            LINE_THICKNESS_EDGE,
            match problem.figure.test_edge_len2(idx, pose) {
                EdgeTestResult::Ok => COLOR_EDGE_OK,
                EdgeTestResult::TooShort => COLOR_EDGE_TOO_SHORT,
                EdgeTestResult::TooLong => COLOR_EDGE_TOO_LONG,
            },
        );
    }

    // Vertices
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
            let dist = NotNan::new(Figure::distance_squared(p, mouse_pos).sqrt()).unwrap();
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
    const VIEWPORT_OFFSET_Y: f32 = 45.0;
    const VIEWPORT_WIDTH: f32 = 600.0;
    const VIEWPORT_HEIGHT: f32 = 600.0;

    let (mut rh, thread) = raylib::init().size(WINDOW_WIDTH, WINDOW_HEIGHT).build();

    let t = Translator::new(
        VIEWPORT_OFFSET_X,
        VIEWPORT_OFFSET_Y,
        VIEWPORT_WIDTH,
        VIEWPORT_HEIGHT,
        &problem,
    );

    let mut gen = solver.solve_gen(&problem, Rc::new(RefCell::new(pose)));
    let pose = gen.resume().unwrap();

    let mut state = GuiState {
        tool: Tool::Move,
        dragged_point: None,
    };

    while !rh.window_should_close() {
        {
            let mut d = rh.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            render_gui(&mut d, &thread, &state, &problem, &pose.borrow());
            render_problem(&mut d, &state, &t, &problem, &pose.borrow());
        }

        if rh.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            let v_idx = hit_test(&pose.borrow(), mouse_pos, 2);
            match state.tool {
                Tool::Move => {
                    state.dragged_point = v_idx;
                }
                Tool::Center => {
                    if let Some(idx) = v_idx {
                        pose.borrow_mut().center(&problem.figure, idx);
                    }
                }
            }
        }

        if rh.is_mouse_button_released(MouseButton::MOUSE_LEFT_BUTTON) {
            state.dragged_point = None;
        }

        if rh.get_gesture_detected() == GestureType::GESTURE_DRAG {
            let mouse_pos = t.untranslate(&rh.get_mouse_position());
            if let Some(idx) = state.dragged_point {
                pose.borrow_mut().vertices[idx] = mouse_pos;
            }
        }

        let mut need_to_sleep = true;
        if let Some(key) = rh.get_key_pressed() {
            match key {
                KeyboardKey::KEY_Q => state.tool = Tool::Move,
                KeyboardKey::KEY_W => state.tool = Tool::Center,
                KeyboardKey::KEY_S => {
                    const PATH: &'static str = "./current.solution";
                    std::fs::write(PATH, pose.borrow().to_json()?)?;
                    info!("Saved to {}", PATH);
                }
                KeyboardKey::KEY_D => {
                    if gen.resume().is_some() {
                        need_to_sleep = false;
                    } else {
                        warn!("No more steps in the solver");
                    }
                }
                _ => {}
            }
        }

        if rh.is_key_down(KeyboardKey::KEY_F) {
            if gen.resume().is_some() {
                need_to_sleep = false;
            } else {
                warn!("No more steps in the solver");
            }
        }

        if need_to_sleep {
            thread::sleep(time::Duration::from_millis(5));
        }
    }
    Ok(())
}
