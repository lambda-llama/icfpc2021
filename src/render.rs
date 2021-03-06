use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::CString;
use std::path::Path;
use std::rc::Rc;
use std::{thread, time};

use geomath::prelude::coordinates::Polar;
use ordered_float::NotNan;
use raylib::prelude::*;

use crate::problem::*;
use crate::solver::Solver;
use crate::transform::Transform;
use crate::{common::*, storage};

struct GuiState {
    // Coord translator
    pub translator: Translator,

    // Dragging
    pub dragged_point: Option<usize>,
    pub viewport_drag_point: Option<Vector2>,
    // Selection
    pub selection_pos: Option<Vector2>,
    pub selected_points: HashSet<usize>,
    // Folding
    pub fold_points: HashSet<usize>,
    // Rotation
    pub rotate_pivot: Option<Point>,
    pub rotate_vertices_copy: Vec<Point>, // to avoid rounding errors with deltas

    // Highlighting
    pub paths: Vec<Vec<usize>>,

    // Problem browser
    pub problems: Vec<CString>,
    pub problems_focus_idx: i32,
    pub problems_scroll_idx: i32,
    pub problems_selected: i32,

    // Solver
    pub solver: &'static Box<dyn Solver>,
}

impl GuiState {
    pub fn new(problem: &Problem, solver: &'static Box<dyn Solver>, id: u32) -> Result<Self> {
        let translator = Self::create_translator(problem);

        let problems_count = storage::get_problems_count();
        let problems = (1..=problems_count)
            .into_iter()
            .map(
                |id| match storage::load_solution(id).expect("Failed to load a solution") {
                    Some(s) => {
                        if s.server_state.dislikes == u64::MAX {
                            CString::new(format!("#152#{}", id)).unwrap()
                        } else if !s.state.valid {
                            CString::new(format!("#150#{}", id)).unwrap()
                        } else if s.state.optimal {
                            CString::new(format!("#157#{}", id)).unwrap()
                        } else {
                            CString::new(format!("{}", id)).unwrap()
                        }
                    }
                    None => CString::new(format!("#152#{}", id)).unwrap(),
                },
            )
            .collect();

        Ok(GuiState {
            translator,
            dragged_point: None,
            viewport_drag_point: None,
            selection_pos: None,
            selected_points: HashSet::new(),
            fold_points: HashSet::new(),
            rotate_pivot: None,
            rotate_vertices_copy: vec![],
            paths: vec![],
            problems,
            problems_focus_idx: 0,
            problems_scroll_idx: 0,
            problems_selected: (id - 1) as i32,
            solver,
        })
    }

    pub fn load_problem(&mut self) -> Result<Problem> {
        let problem = storage::load_problem((self.problems_selected + 1) as u32)?;
        self.translator = Self::create_translator(&problem);
        self.dragged_point = None;
        self.viewport_drag_point = None;
        self.selection_pos = None;
        self.selected_points.clear();
        self.fold_points.clear();
        self.rotate_pivot = None;
        self.rotate_vertices_copy.clear();
        self.paths.clear();
        Ok(problem)
    }

    fn create_translator(problem: &Problem) -> Translator {
        const MARGIN: f32 = 75.0;
        const VIEWPORT_OFFSET_X: f32 = MARGIN;
        const VIEWPORT_OFFSET_Y: f32 = MARGIN;
        const VIEWPORT_WIDTH: f32 = 1024.0 - 2.0 * MARGIN;
        const VIEWPORT_HEIGHT: f32 = 768.0 - 2.0 * MARGIN;
        let translator = Translator::new(
            VIEWPORT_OFFSET_X,
            VIEWPORT_OFFSET_Y,
            VIEWPORT_WIDTH,
            VIEWPORT_HEIGHT,
            problem,
        );
        translator
    }

    pub fn translate(&self, p: &Point) -> Vector2 {
        self.translator.translate(p)
    }

    pub fn untranslate(&self, v: &Vector2) -> Point {
        self.translator.untranslate(v)
    }
}

struct Translator {
    x_offset: f32,
    y_offset: f32,
    zero: Point,
    max: Point,
    step: f32,
}

impl Translator {
    fn new(x_offset: f32, y_offset: f32, width: f32, height: f32, p: &Problem) -> Translator {
        let (min_p, max_p) = p.bounding_box();
        let x_step = width / ((max_p.x - min_p.x) as f32);
        let y_step = height / ((max_p.y - min_p.y) as f32);
        let step = x_step.min(y_step);
        return Translator {
            x_offset,
            y_offset,
            zero: min_p,
            max: max_p,
            step,
        };
    }

    fn translate(&self, p: &Point) -> Vector2 {
        return Vector2::new(
            ((p.x - self.zero.x) as f32) * self.step + self.x_offset,
            ((p.y - self.zero.y) as f32) * self.step + self.y_offset,
        );
    }

    fn untranslate(&self, v: &Vector2) -> Point {
        return Point {
            x: ((v.x - self.x_offset) / self.step + (self.zero.x as f32)).round() as i64,
            y: ((v.y - self.y_offset) / self.step + (self.zero.y as f32)).round() as i64,
        };
    }
}

fn render_gui(
    d: &mut RaylibDrawHandle,
    thread: &RaylibThread,
    state: &mut GuiState,
    problem: &Problem,
    pose: &Pose,
) -> i32 {
    // Window title
    d.set_window_title(
        &thread,
        &format!(
            "Problem {}; eps: {}; dlike_score: {}; inside: {}, edges ok: {}",
            problem.id,
            problem.figure.epsilon,
            problem.dislikes(pose),
            problem.contains(pose),
            problem.correct_length(pose)
        ),
    );

    // Status bar
    const STATUS_BAR_HEIGHT: f32 = 25.0;
    let tool = if d.is_key_down(KeyboardKey::KEY_R) {
        "Rotate"
    } else if d.is_key_down(KeyboardKey::KEY_W) {
        "Fold"
    } else {
        "Move"
    };
    let text = format!("Tool: {}", tool);
    d.gui_status_bar(
        Rectangle {
            x: 0.0,
            y: 0.0,
            width: d.get_screen_width() as f32,
            height: STATUS_BAR_HEIGHT,
        },
        Some(&CString::new(text).unwrap()),
    );

    // Help bar
    const HELP_BAR_HEIGHT: f32 = 51.0;
    let mut text = b"\
Tools: Q - Pull, Shift+Q - Push, E - Center Illegal, Shift+E - Center All, C - Flip Horz, V - Flip Vert, W - Fold (hold), R - Rotate (hold)\n\
Selection/Navigation: Ctrl+A - Select All, Shift adds, Ctrl removes, Z - Select Adjacent, X - Invert Selection, RMB - Drag Viewport, Scrollwheel - Zoom
Misc: S - Save, D - Step Solver, F - Run Solver, Shift+L - Reset Selected, Ctrl+L - Reset Solution\n\
"
    .to_owned();
    d.gui_text_box_multi(
        Rectangle {
            x: 0.0,
            y: d.get_screen_height() as f32 - HELP_BAR_HEIGHT,
            width: d.get_screen_width() as f32,
            height: HELP_BAR_HEIGHT,
        },
        &mut text,
        false,
    );

    // Problem selector
    const PROBLEM_SELECTOR_WIDTH: f32 = 60.0;
    let problems = state
        .problems
        .iter()
        .map(|s| s.as_c_str())
        .collect::<Vec<_>>();
    let selected_problem = d.gui_list_view_ex(
        Rectangle {
            x: d.get_screen_width() as f32 - PROBLEM_SELECTOR_WIDTH,
            y: STATUS_BAR_HEIGHT,
            width: PROBLEM_SELECTOR_WIDTH,
            height: d.get_screen_height() as f32 - STATUS_BAR_HEIGHT - HELP_BAR_HEIGHT,
        },
        &problems[..],
        &mut state.problems_focus_idx,
        &mut state.problems_scroll_idx,
        state.problems_selected,
    );

    // Selection window
    if let Some(pos) = state.selection_pos {
        let rect = vec2_to_rect(pos, d.get_mouse_position());
        d.draw_rectangle_lines_ex(rect, 1, Color::DARKGRAY);
    }

    // Rotation widget
    if let Some(p) = state.rotate_pivot {
        let pt = state.translate(&p);
        let mouse_pos = d.get_mouse_position();
        let mut vec =
            geomath::vector::Vector2::new((mouse_pos.x - pt.x) as f64, (mouse_pos.y - pt.y) as f64);
        let angle = vec.phi();
        vec.set_phi(0.0);
        vec.set_rho(60.0);
        d.draw_circle_sector_lines(
            pt,
            40.0,
            90,
            90 - (angle / std::f64::consts::PI * 180.0) as i32,
            36,
            Color::GREEN,
        );
        d.draw_line_v(
            pt,
            Vector2::new(pt.x + vec.x as f32, pt.y + vec.y as f32),
            Color::BLUE,
        );
        d.draw_line_v(pt, mouse_pos, Color::BLUE);
    }

    selected_problem
}

fn render_problem(d: &mut RaylibDrawHandle, state: &GuiState, problem: &Problem, pose: &Pose) {
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
    const COLOR_VERTEX_SELECTED2: Color = Color::MAGENTA;
    const COLOR_EDGE_OK: Color = Color::GREEN;
    const COLOR_EDGE_TOO_SHORT: Color = Color::BLUE;
    const COLOR_EDGE_TOO_LONG: Color = Color::RED;
    const COLOR_EDGE_HIGHLIGHT: Color = Color::MAGENTA;

    // Grid
    let connected_edge_bounds = state.dragged_point.map(|v_idx| {
        problem.figure.vertex_edges[v_idx]
            .iter()
            .map(|&(e, v)| (v, problem.figure.edge_len2_bounds(e)))
            .collect::<Vec<_>>()
    });
    for x in state.translator.zero.x..state.translator.max.x {
        for y in state.translator.zero.y..state.translator.max.y {
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
            let grid_point = state.translate(&Point { x, y });
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
        d.draw_circle_v(state.translate(&p), POINT_RADIUS, COLOR_HOLE);
        match last_p {
            Some(pp) => d.draw_line_ex(
                state.translate(&pp),
                state.translate(&p),
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
            state.translate(&b.position),
            POINT_RADIUS_BONUS_UNLOCK,
            COLOR_BONUS_UNLOCK,
        );
    }

    // Edges
    for (idx, e) in problem.figure.edges.iter().enumerate() {
        let color = {
            let mut color = None;
            for path in &state.paths {
                let pos0 = path.iter().position(|&v| v == e.v0);
                let pos1 = path.iter().position(|&v| v == e.v1);
                match (pos0, pos1) {
                    (Some(i), Some(j)) if (i as i32 - j as i32).abs() == 1 => {
                        color = Some(COLOR_EDGE_HIGHLIGHT);
                        break;
                    }
                    _ => {}
                }
            }
            color.unwrap_or_else(|| match problem.figure.test_edge_len2(idx, pose) {
                EdgeTestResult::Ok => COLOR_EDGE_OK,
                EdgeTestResult::TooShort => COLOR_EDGE_TOO_SHORT,
                EdgeTestResult::TooLong => COLOR_EDGE_TOO_LONG,
            })
        };
        d.draw_line_ex(
            state.translate(&pose.vertices[e.v0 as usize]),
            state.translate(&pose.vertices[e.v1 as usize]),
            LINE_THICKNESS_EDGE,
            color,
        );
    }

    // Vertices
    for (idx, p) in pose.vertices.iter().enumerate() {
        let color = if state.fold_points.contains(&idx) {
            COLOR_VERTEX_SELECTED2
        } else if state.selected_points.contains(&idx) {
            COLOR_VERTEX_SELECTED
        } else {
            COLOR_VERTEX
        };
        d.draw_circle_v(state.translate(p), POINT_RADIUS, color);
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

// TODO: remove duplication
fn hit_test_hole(problem: &Problem, mouse_pos: Point, dist: i64) -> Option<usize> {
    let mut points_with_dist = problem
        .hole
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

pub fn interact<'a>(
    solution_path: Option<&Path>,
    solver: &'static Box<dyn Solver>,
    id: u32,
) -> Result<()> {
    use raylib::consts::*;

    const WINDOW_WIDTH: i32 = 1024;
    const WINDOW_HEIGHT: i32 = 768;

    let (mut rh, thread) = raylib::init().size(WINDOW_WIDTH, WINDOW_HEIGHT).build();

    let mut problem = storage::load_problem(id)?;
    let mut state = GuiState::new(&problem, solver, id)?;

    let pose = match solution_path {
        Some(p) => storage::load_custom_solution(p)?,
        None => {
            let solution = storage::load_solution(id)?;
            solution
                .map(|s| s.pose)
                .unwrap_or_else(|| problem.figure.get_default_pose())
        }
    };

    let mut gen = state
        .solver
        .solve_gen(problem.clone(), Rc::new(RefCell::new(pose)));
    let mut pose = gen.resume().unwrap();

    while !rh.window_should_close() {
        {
            let mut d = rh.begin_drawing(&thread);
            d.clear_background(Color::WHITE);
            render_problem(&mut d, &state, &problem, &pose.borrow());
            let selected_problem =
                render_gui(&mut d, &thread, &mut state, &problem, &pose.borrow());

            if selected_problem != -1 && state.problems_selected != selected_problem {
                state.problems_selected = selected_problem;
                problem = state.load_problem()?;
                let solution = storage::load_solution(problem.id)?;
                let initial_pose = solution
                    .map(|s| s.pose)
                    .unwrap_or_else(|| problem.figure.get_default_pose());
                gen = state
                    .solver
                    .solve_gen(problem.clone(), Rc::new(RefCell::new(initial_pose)));
                pose = gen.resume().unwrap();
            }
        }

        let mouse_pos = rh.get_mouse_position();

        if rh.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_p = state.untranslate(&mouse_pos);
            let mut continue_processing = true;
            let v_idx = hit_test_point(&pose.borrow(), mouse_p, 2);
            let h_idx = hit_test_hole(&problem, mouse_p, 2);
            if h_idx.is_some() && !v_idx.is_some() {
                let h_idx = h_idx.unwrap();
                let mut desired = vec![];
                for i in 0..problem.hole.len() {
                    let h1 = (h_idx + i) % problem.hole.len();
                    let h2 = (h_idx + i + 1) % problem.hole.len();
                    desired.push(Figure::distance_squared(problem.hole[h1], problem.hole[h2]));
                }
                state.paths = problem.figure.get_longest_edge_paths(&desired);
                state.selected_points.clear();
                for path in &state.paths {
                    for v in path {
                        state.selected_points.insert(*v);
                    }
                }
                if rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) && state.paths.len() > 0 {
                    for (i, &v_idx) in state.paths[0].iter().enumerate() {
                        pose.borrow_mut().vertices[v_idx] =
                            problem.hole[(h_idx + i) % problem.hole.len()];
                    }
                    state.paths.clear();
                    continue_processing = false;
                }
            }
            if continue_processing {
                if rh.is_key_down(KeyboardKey::KEY_R) {
                    state.rotate_pivot = Some(mouse_p);
                    state.rotate_vertices_copy = pose.borrow().vertices.clone();
                } else if rh.is_key_down(KeyboardKey::KEY_W) {
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
                } else {
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
            }
        } else if rh.is_mouse_button_pressed(MouseButton::MOUSE_RIGHT_BUTTON) {
            state.viewport_drag_point = Some(mouse_pos);
        } else {
            if mouse_pos.x < (rh.get_screen_width() as f32 - 50.0) {
                let scroll = rh.get_mouse_wheel_move();
                let scroll_amount = 0.95;
                if scroll.abs() > 0.5 {
                    if scroll > 0.0 {
                        let offset = 2.0 * state.translator.step / scroll_amount;
                        state.translator.x_offset -= offset;
                        state.translator.y_offset -= offset;
                        state.translator.step /= scroll_amount;
                    } else {
                        let offset = 2.0 * state.translator.step * scroll_amount;
                        state.translator.x_offset += offset;
                        state.translator.y_offset += offset;
                        state.translator.step *= scroll_amount;
                    }
                }
            }
        }

        if rh.is_mouse_button_released(MouseButton::MOUSE_LEFT_BUTTON) {
            state.dragged_point = None;
            state.rotate_pivot = None;
            state.rotate_vertices_copy.clear();
            if let Some(pos) = state.selection_pos {
                let rect = vec2_to_rect(pos, mouse_pos);
                let min = state.untranslate(&Vector2 {
                    x: rect.x,
                    y: rect.y,
                });
                let max = state.untranslate(&Vector2 {
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
        } else if rh.is_mouse_button_released(MouseButton::MOUSE_RIGHT_BUTTON) {
            state.viewport_drag_point = None;
        }

        if rh.get_gesture_detected() == GestureType::GESTURE_DRAG
            || rh.is_mouse_button_down(MouseButton::MOUSE_LEFT_BUTTON)
            || rh.is_mouse_button_down(MouseButton::MOUSE_RIGHT_BUTTON)
        {
            if let Some(p) = state.viewport_drag_point {
                let delta = mouse_pos - p;
                state.translator.x_offset += delta.x;
                state.translator.y_offset += delta.y;
                state.viewport_drag_point = Some(mouse_pos);
            } else {
                let mouse_p = state.untranslate(&mouse_pos);
                if let Some(idx) = state.dragged_point {
                    let diff_p = mouse_p - pose.borrow().vertices[idx];
                    let vertices = &mut pose.borrow_mut().vertices;
                    for &idx in state.selected_points.iter() {
                        vertices[idx] = vertices[idx] + diff_p;
                    }
                } else if let Some(p) = state.rotate_pivot {
                    let angle = geomath::vector::Vector2::new(
                        (mouse_p.x - p.x) as f64,
                        (mouse_p.y - p.y) as f64,
                    )
                    .phi();
                    for &idx in state.selected_points.iter() {
                        // We need to restore the original point and rotate it to avoid
                        // rounding errors due to the float angle rotation of int coords
                        pose.borrow_mut().vertices[idx] = state.rotate_vertices_copy[idx];
                        pose.borrow_mut().rotate(idx, p, angle);
                    }
                }
            }
        }

        let mut need_to_sleep = true;
        if let Some(key) = rh.get_key_pressed() {
            match key {
                KeyboardKey::KEY_Q => {
                    if rh.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
                        pose.borrow_mut()
                            .push(&problem.figure, state.selected_points.clone());
                    } else {
                        for &idx in &state.selected_points {
                            pose.borrow_mut().pull(&problem.figure, idx);
                        }
                    }
                }
                KeyboardKey::KEY_E => {
                    let points = if rh.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
                        state.selected_points.iter().cloned().collect::<Vec<_>>()
                    } else {
                        // If Shift is not pressed, only process points that have illegal edges
                        let mut points_with_ill_edge_counts = state
                            .selected_points
                            .iter()
                            .map(|&idx| {
                                let illegal_edges_count = problem.figure.vertex_edges[idx]
                                    .iter()
                                    .filter(|&&(e, _v)| {
                                        problem.figure.test_edge_len2(e, &pose.borrow())
                                            != EdgeTestResult::Ok
                                    })
                                    .count();
                                (idx, illegal_edges_count)
                            })
                            .filter(|&(_idx, count)| count > 0)
                            .collect::<Vec<_>>();
                        points_with_ill_edge_counts
                            .sort_unstable_by_key(|&(_idx, count)| usize::MAX - count);
                        points_with_ill_edge_counts
                            .into_iter()
                            .map(|(idx, _count)| idx)
                            .collect()
                    };
                    for idx in points {
                        pose.borrow_mut()
                            .center(&problem.figure, idx, problem.bounding_box());
                    }
                }
                KeyboardKey::KEY_Z => {
                    let existing_points = state.selected_points.clone();
                    for idx in existing_points {
                        for &(_e, v) in &problem.figure.vertex_edges[idx] {
                            state.selected_points.insert(v);
                        }
                    }
                }
                KeyboardKey::KEY_X => {
                    let current = state.selected_points.clone();
                    state.selected_points.clear();
                    for i in 0..problem.figure.vertices.len() {
                        if !current.contains(&i) {
                            state.selected_points.insert(i);
                        }
                    }
                }
                KeyboardKey::KEY_C => {
                    for &idx in state.selected_points.iter() {
                        pose.borrow_mut().flip_h(idx, problem.bounding_box());
                    }
                }
                KeyboardKey::KEY_V => {
                    for &idx in state.selected_points.iter() {
                        pose.borrow_mut().flip_v(idx, problem.bounding_box());
                    }
                }
                KeyboardKey::KEY_A if rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) => {
                    for idx in 0..problem.figure.vertices.len() {
                        state.selected_points.insert(idx);
                    }
                }
                KeyboardKey::KEY_S => {
                    let id = (state.problems_selected + 1) as u32;
                    let dislikes = problem.dislikes(&pose.borrow());
                    let s = SolutionState {
                        dislikes,
                        valid: problem.validate(&pose.borrow()),
                        optimal: dislikes == 0,
                    };
                    let solution = Solution {
                        id,
                        pose: pose.borrow().clone(),
                        state: s,
                        server_state: storage::load_server_state(id)?,
                    };
                    storage::save_solution(&solution, None)?;
                    info!("Saved solution {} to the default solution folder", id);
                }
                KeyboardKey::KEY_D => {
                    if let Some(temp) = gen.resume() {
                        pose = temp;
                        need_to_sleep = false;
                    } else {
                        warn!("No more steps in the solver");
                    }
                }
                KeyboardKey::KEY_L => {
                    if rh.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
                        for &idx in &state.selected_points {
                            pose.borrow_mut().vertices[idx] = problem.figure.vertices[idx];
                        }
                    } else if rh.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) {
                        gen = state.solver.solve_gen(
                            problem.clone(),
                            Rc::new(RefCell::new(problem.figure.get_default_pose())),
                        );
                        pose = gen.resume().unwrap();
                    }
                }
                _ => {}
            }
        }

        if rh.is_key_down(KeyboardKey::KEY_F) {
            if let Some(temp) = gen.resume() {
                pose = temp;
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
