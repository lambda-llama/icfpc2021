use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::CString;
use std::rc::Rc;
use std::{thread, time};

use ordered_float::NotNan;
use raylib::prelude::*;

use crate::common::*;
use crate::problem::*;
use crate::solver::Solver;
use crate::transform::Transform;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tool {
    Move,
    Center,
    Fold,
}

struct GuiState {
    pub tool: Tool,
    pub dragged_point: Option<usize>,
    pub selection_pos: Option<Vector2>,
    pub selected_points: HashSet<usize>,
    pub fold_points: HashSet<usize>,
}

impl GuiState {
    pub fn switch_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.dragged_point = None;
        self.selection_pos = None;
        self.selected_points.clear();
        self.fold_points.clear();
    }
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
            problem.contains(pose),
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
    let mut text = b"\
Tools: Q - Move, W - Center\n\
Selection: Ctrl+A - Select all, Shift adds, Ctrl removes
Misc: S - Save, D - Step solver, F - Run solver\n\
"
    .to_owned();
    d.gui_text_box_multi(
        Rectangle {
            x: 0.0,
            y: d.get_screen_height() as f32 - 51.0,
            width: d.get_screen_width() as f32,
            height: 51.0,
        },
        &mut text,
        false,
    );

    // Selection window
    if let Some(pos) = state.selection_pos {
        let rect = vec2_to_rect(pos, d.get_mouse_position());
        d.draw_rectangle_lines_ex(rect, 1, Color::DARKGRAY);
    }
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
    const COLOR_VERTEX_SELECTED: Color = Color::GREEN;
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
    for (idx, p) in pose.vertices.iter().enumerate() {
        let color = if state.selected_points.contains(&idx) || state.fold_points.contains(&idx) {
            COLOR_VERTEX_SELECTED
        } else {
            COLOR_VERTEX
        };
        d.draw_circle_v(t.translate(p), POINT_RADIUS, color);
    }
}

fn hit_test_point(pose: &Pose, mouse_pos: Point, dist: i64) -> Option<usize> {
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

fn hit_test_rect(pose: &Pose, min: Point, max: Point) -> Vec<usize> {
    pose.vertices
        .iter()
        .enumerate()
        .filter(|&(_i, &p)| p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y)
        .map(|(i, _p)| i)
        .collect()
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
        selection_pos: None,
        selected_points: HashSet::new(),
        fold_points: HashSet::new(),
    };

    while !rh.window_should_close() {
        {
            let mut d = rh.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            render_problem(&mut d, &state, &t, &problem, &pose.borrow());
            render_gui(&mut d, &thread, &state, &problem, &pose.borrow());
        }

        let mouse_pos = rh.get_mouse_position();

        if rh.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_p = t.untranslate(&mouse_pos);
            let v_idx = hit_test_point(&pose.borrow(), mouse_p, 2);
            match state.tool {
                Tool::Move => {
                    state.dragged_point = v_idx;
                    if let Some(idx) = v_idx {
                        if !rh.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                            && !rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                            && !state.selected_points.contains(&idx)
                        {
                            state.selected_points.clear();
                        }
                        if rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
                            state.selected_points.remove(&idx);
                        } else {
                            state.selected_points.insert(idx);
                        }
                    } else {
                        state.selection_pos = Some(mouse_pos);
                    }
                }
                Tool::Center => {
                    if let Some(idx) = v_idx {
                        pose.borrow_mut().center(&problem.figure, idx);
                    }
                }
                Tool::Fold => {
                    if let Some(idx) = v_idx {
                        if state.fold_points.contains(&idx) {
                            state.fold_points.remove(&idx);
                        } else {
                            if state.fold_points.len() < 2 {
                                state.fold_points.insert(idx);
                            } else {
                                let mut points =
                                    state.fold_points.iter().cloned().collect::<Vec<_>>();
                                points.sort_unstable();
                                pose.borrow_mut()
                                    .fold(&problem.figure, points[0], points[1], idx);
                                state.fold_points.clear();
                            }
                        }
                    }
                }
            }
        }

        if rh.is_mouse_button_released(MouseButton::MOUSE_LEFT_BUTTON) {
            state.dragged_point = None;
            if let Some(pos) = state.selection_pos {
                let rect = vec2_to_rect(pos, mouse_pos);
                let min = t.untranslate(&Vector2 {
                    x: rect.x,
                    y: rect.y,
                });
                let max = t.untranslate(&Vector2 {
                    x: rect.x + rect.width,
                    y: rect.y + rect.height,
                });
                let hits = hit_test_rect(&pose.borrow(), min, max);
                if !rh.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                    && !rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                {
                    state.selected_points.clear();
                }
                if rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
                    for hit in hits {
                        state.selected_points.remove(&hit);
                    }
                } else {
                    for hit in hits {
                        state.selected_points.insert(hit);
                    }
                }
                state.selection_pos = None;
            }
        }

        if rh.get_gesture_detected() == GestureType::GESTURE_DRAG {
            let mouse_p = t.untranslate(&mouse_pos);
            if let Some(idx) = state.dragged_point {
                let diff_p = mouse_p - pose.borrow().vertices[idx];
                let vertices = &mut pose.borrow_mut().vertices;
                for &idx in state.selected_points.iter() {
                    vertices[idx] = vertices[idx] + diff_p;
                }
            }
        }

        let mut need_to_sleep = true;
        if let Some(key) = rh.get_key_pressed() {
            match key {
                KeyboardKey::KEY_Q => state.switch_tool(Tool::Move),
                KeyboardKey::KEY_W => state.switch_tool(Tool::Center),
                KeyboardKey::KEY_E => state.switch_tool(Tool::Fold),
                KeyboardKey::KEY_A
                    if rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL)
                        && state.tool == Tool::Move =>
                {
                    for idx in 0..problem.figure.vertices.len() {
                        state.selected_points.insert(idx);
                    }
                }
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

fn vec2_to_rect(v1: Vector2, v2: Vector2) -> Rectangle {
    let mut min_x = v1.x;
    let mut max_x = v1.x;
    let mut min_y = v1.y;
    let mut max_y = v1.y;
    if v2.x < min_x {
        min_x = v2.x;
    }
    if v2.x > max_x {
        max_x = v2.x;
    }
    if v2.y < min_y {
        min_y = v2.y;
    }
    if v2.y > max_y {
        max_y = v2.y;
    }
    Rectangle::new(min_x, min_y, max_x - min_x, max_y - min_y)
}
