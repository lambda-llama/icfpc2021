use geo::algorithm::contains::Contains;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::algorithm::line_intersection::{line_intersection, LineIntersection};
use geo::relate::Relate;
use ordered_float::NotNan;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::common::*;

pub type Point = geo::Coordinate<i64>;

#[derive(Clone, Debug)]
pub struct Edge {
    pub v0: usize,
    pub v1: usize,
    pub len2: f64,
}

#[derive(Clone, Debug)]
pub struct Figure {
    pub vertices: Vec<Point>,
    pub edges: Vec<Edge>,
    pub vertex_edges: Vec<Vec<(usize, usize)>>,
    pub epsilon: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeTestResult {
    Ok,
    TooShort,
    TooLong,
}

fn bounding_box(vertices: &Vec<Point>) -> (Point, Point) {
    let mut min_p = Point {
        x: i64::MAX,
        y: i64::MAX,
    };
    let mut max_p = Point { x: 0, y: 0 };
    for p in vertices {
        min_p.x = std::cmp::min(min_p.x, p.x);
        max_p.x = std::cmp::max(max_p.x, p.x);
        min_p.y = std::cmp::min(min_p.y, p.y);
        max_p.y = std::cmp::max(max_p.y, p.y);
    }
    return (min_p, max_p);
}

fn is_point_belongs_to_poly(poly: &geo::Polygon<f64>, p: Point) -> bool {
    let relation = poly.relate(&p.convert());
    relation.is_within() || relation.is_intersects()
}

fn is_segment_belongs_to_poly(poly: &geo::Polygon<f64>, (a, b): (Point, Point)) -> bool {
    let s = geo::LineString::from(vec![a.convert(), b.convert()]);
    let polygon_points = poly.exterior().clone().into_points();
    let mut boundary_countains = false;
    for i in 0..polygon_points.len() - 1 {
        let t = geo::LineString::from(vec![polygon_points[i], polygon_points[i + 1]]);
        if t.contains(&s) {
            boundary_countains = true;
        }
    }
    boundary_countains || poly.contains(&s)
}

impl Figure {
    pub fn new(vertices: Vec<Point>, edges: Vec<Edge>, epsilon: f64) -> Self {
        let mut vertex_edges = vec![Vec::new(); vertices.len()];
        for (i, e) in edges.iter().enumerate() {
            vertex_edges[e.v0].push((i, e.v1));
            vertex_edges[e.v1].push((i, e.v0));
        }

        Self {
            vertices,
            edges,
            vertex_edges,
            epsilon,
        }
    }

    pub fn distance_squared(p: Point, q: Point) -> f64 {
        ((p.x - q.x) as f64).powi(2) + ((p.y - q.y) as f64).powi(2)
    }

    pub fn distance_squared_int(p: Point, q: Point) -> i64 {
        (p.x - q.x).pow(2) + (p.y - q.y).pow(2)
    }

    pub fn edge_len2(&self, idx: usize, pose: &Pose) -> f64 {
        let e = &self.edges[idx];
        let p = pose.vertices[e.v0];
        let q = pose.vertices[e.v1];
        Self::distance_squared(p, q)
    }

    pub fn edge_len2_bounds(&self, idx: usize) -> (f64, f64) {
        let len2_default = self.edges[idx].len2;
        (
            (1.0f64 - self.epsilon) * len2_default,
            (1.0f64 + self.epsilon) * len2_default,
        )
    }

    pub fn edge_len2_bounds_int(&self, idx: usize) -> (i64, i64) {
        let len2_default = self.edges[idx].len2;
        (
            ((1.0f64 - self.epsilon) * len2_default).ceil() as i64,
            ((1.0f64 + self.epsilon) * len2_default).floor() as i64,
        )
    }

    pub fn edge_len2_diff(&self, idx: usize, pose: &Pose) -> f64 {
        self.edge_len2(idx, pose) - self.edges[idx].len2
    }

    pub fn test_edge_len2(&self, idx: usize, pose: &Pose) -> EdgeTestResult {
        let diff = self.edge_len2_diff(idx, pose);
        let allowed = self.epsilon * self.edges[idx].len2;
        if diff.abs() <= allowed {
            EdgeTestResult::Ok
        } else if diff < 0.0 {
            EdgeTestResult::TooShort
        } else {
            EdgeTestResult::TooLong
        }
    }

    pub fn to_float_point(p: Point) -> geo::Point<f64> {
        geo::Point::new(p.x as f64, p.y as f64)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BonusType {
    Globalist, // Shared epsilon
    BreakALeg, // Divide an edge into two
}

impl From<&str> for BonusType {
    fn from(b: &str) -> Self {
        match b {
            "GLOBALIST" => BonusType::Globalist,
            "BREAK_A_LEG" => BonusType::BreakALeg,
            t => panic!("Failed to parse bonus type '{}'", t),
        }
    }
}

impl From<BonusType> for String {
    fn from(b: BonusType) -> Self {
        match b {
            BonusType::Globalist => "GLOBALIST".to_owned(),
            BonusType::BreakALeg => "BREAK_A_LEG".to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BonusUnlock {
    pub position: Point,
    pub bonus: BonusType,
    pub problem: u32,
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub hole: Vec<Point>,
    pub poly: geo::Polygon<f64>,
    inside_points: Vec<Vec<bool>>,
    inside_segments: HashSet<(Point, Point)>,
    pub figure: Figure,
    pub bonuses: Vec<BonusUnlock>,
}

impl Problem {
    pub fn new(hole: Vec<Point>, figure: Figure, bonuses: Vec<BonusUnlock>) -> Self {
        let mut border: Vec<geo::Coordinate<f64>> = hole
            .clone()
            .into_iter()
            .map(|p| geo::Coordinate {
                x: p.x as f64,
                y: p.y as f64,
            })
            .collect();
        border.push(border[0]);
        let poly = geo::Polygon::new(geo::LineString::from(border), vec![]);

        let (mn, mx) = bounding_box(&hole);
        let mut inside_points = vec![vec![false; mx.y as usize + 1]; mx.x as usize + 1];
        let mut inside_segments = HashSet::new();
        for x in mn.x..=mx.x {
            for y in mn.y..=mx.y {
                let p = Point { x, y };
                if is_point_belongs_to_poly(&poly, p) {
                    inside_points[p.x as usize][p.y as usize] = true;
                    // TODO: Currently it slows down startup of the render mode. We need to do it in a lazy way.
                    // for &q in &inside_points {
                    //     if is_segment_belongs_to_poly(&poly, (p, q)) {
                    //         inside_segments.insert((p, q));
                    //     }
                    // }
                    // inside_points.insert(p);
                }
            }
        }

        Self {
            hole: hole,
            poly: poly,
            inside_points: inside_points,
            inside_segments: inside_segments,
            figure: figure,
            bonuses,
        }
    }

    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawProblem {
            hole,
            figure: RawFigure { vertices, edges },
            epsilon,
            bonuses,
        } = serde_json::from_slice(data)?;
        let vertices = vertices
            .into_iter()
            .map(|p| Point { x: p[0], y: p[1] })
            .collect::<Vec<_>>();
        let edges = edges
            .into_iter()
            .map(|e| Edge {
                v0: e[0] as usize,
                v1: e[1] as usize,
                len2: Figure::distance_squared(vertices[e[0] as usize], vertices[e[1] as usize]),
            })
            .collect();
        let bonuses = bonuses
            .into_iter()
            .map(|b| -> BonusUnlock {
                BonusUnlock {
                    position: Point {
                        x: b.position[0],
                        y: b.position[1],
                    },
                    bonus: b.bonus[..].into(),
                    problem: b.problem,
                }
            })
            .collect();
        Ok(Problem::new(
            hole.into_iter()
                .map(|p| Point { x: p[0], y: p[1] })
                .collect(),
            Figure::new(vertices, edges, epsilon as f64 / 1_000_000.0f64),
            bonuses,
        ))
    }

    pub fn dislikes(&self, pose: &Pose) -> u64 {
        let sum: f64 = self
            .hole
            .iter()
            .map(|&v| {
                pose.vertices
                    .iter()
                    .map(|&p| NotNan::new(Figure::distance_squared(p, v)).unwrap())
                    .min()
                    .unwrap()
                    .into_inner()
            })
            .sum();
        sum.trunc() as u64
    }

    pub fn contains(&self, pose: &Pose) -> bool {
        // 1 - vertices are inside
        for &p in &pose.vertices {
            if !is_point_belongs_to_poly(&self.poly, p) {
                return false;
            }
        }
        // 2 - edges are inside
        for e in &self.figure.edges {
            if !is_segment_belongs_to_poly(&self.poly, (pose.vertices[e.v0], pose.vertices[e.v1])) {
                return false;
            }
        }
        true
    }

    pub fn contains_point(&self, p: &Point) -> bool {
        if p.x < 0
            || p.x >= self.inside_points.len() as i64
            || p.y < 0
            || p.y >= self.inside_points[0].len() as i64
        {
            return false;
        }
        self.inside_points[p.x as usize][p.y as usize]
    }

    pub fn contains_segment(&self, (a, b): (Point, Point)) -> bool {
        self.inside_segments.contains(&(a, b)) || self.inside_segments.contains(&(b, a))
    }

    pub fn correct_length(&self, pose: &Pose) -> bool {
        for idx in 0..self.figure.edges.len() {
            if self.figure.test_edge_len2(idx, pose) != EdgeTestResult::Ok {
                return false;
            }
        }
        true
    }

    pub fn validate(&self, pose: &Pose) -> bool {
        self.contains(&pose) && self.correct_length(&pose)
    }

    pub fn min_distance_to(&self, point: Point) -> f64 {
        let p = point.convert();
        if self.poly.contains(&p) {
            return 0.0;
        }
        return self.poly.euclidean_distance(&p);
    }

    pub fn edge_intersections(&self, src_pos: Point, dst_pos: Point) -> f64 {
        let edge = geo::Line::new(src_pos.convert(), dst_pos.convert());
        if self.poly.contains(&edge) {
            return 0.0;
        }

        let mut intersections = 0.0;
        for poly_line in self.poly.exterior().lines() {
            match line_intersection(poly_line, edge) {
                None => {}
                Some(LineIntersection::SinglePoint {
                    intersection,
                    is_proper,
                }) => {
                    if is_proper {
                        let int_point: geo::Point<f64> = intersection.into();
                        intersections += std::cmp::min(
                            std::cmp::min(
                                NotNan::new(int_point.euclidean_distance(&poly_line.start_point()))
                                    .unwrap(),
                                NotNan::new(int_point.euclidean_distance(&poly_line.end_point()))
                                    .unwrap(),
                            ),
                            std::cmp::min(
                                NotNan::new(int_point.euclidean_distance(&edge.start_point()))
                                    .unwrap(),
                                NotNan::new(int_point.euclidean_distance(&edge.end_point()))
                                    .unwrap(),
                            ),
                        )
                        .into_inner();
                    }
                }
                Some(LineIntersection::Collinear { intersection }) => {
                    if !poly_line.contains(&edge) {
                        intersections += intersection
                            .start_point()
                            .euclidean_distance(&intersection.end_point());
                    }
                }
            }
        }
        return intersections;
    }

    pub fn bounding_box(&self) -> (Point, Point) {
        bounding_box(&self.hole)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BonusUse {
    pub bonus: BonusType,
    pub problem: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Pose {
    pub vertices: Vec<Point>,
    pub bonuses: Vec<BonusUse>,
}

impl Pose {
    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawPose { vertices, bonuses } = serde_json::from_slice(data)?;
        Ok(Pose {
            vertices: vertices
                .into_iter()
                .map(|p| Point { x: p[0], y: p[1] })
                .collect(),
            bonuses: bonuses
                .into_iter()
                .map(|b| BonusUse {
                    bonus: b.bonus[..].into(),
                    problem: b.problem,
                })
                .collect(),
        })
    }

    pub fn to_json(&self) -> Result<String> {
        let pose = RawPose {
            vertices: self.vertices.iter().map(|p| vec![p.x, p.y]).collect(),
            bonuses: self
                .bonuses
                .iter()
                .map(|b| RawBonusUse {
                    bonus: b.bonus.into(),
                    problem: b.problem,
                })
                .collect(),
        };
        Ok(serde_json::to_string(&pose)?)
    }
}

pub struct SolutionState {
    pub dislikes: u64,
}

impl SolutionState {
    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawSolutionState { dislikes } = serde_json::from_slice(data)?;
        Ok(SolutionState { dislikes })
    }

    pub fn to_json(&self) -> Result<String> {
        let solution_state = RawSolutionState {
            dislikes: self.dislikes,
        };
        Ok(serde_json::to_string(&solution_state)?)
    }
}

// Serialization helper types below

#[derive(Deserialize)]
struct RawFigure {
    pub vertices: Vec<Vec<i64>>,
    pub edges: Vec<Vec<u64>>,
}

#[derive(Deserialize)]
struct RawProblem {
    pub hole: Vec<Vec<i64>>,
    pub figure: RawFigure,
    pub epsilon: u64,
    pub bonuses: Vec<RawBonusUnlock>,
}

#[derive(Deserialize, Serialize)]
struct RawPose {
    pub vertices: Vec<Vec<i64>>,
    #[serde(default)]
    pub bonuses: Vec<RawBonusUse>,
}

#[derive(Deserialize)]
struct RawBonusUnlock {
    pub position: Vec<i64>,
    pub bonus: String,
    pub problem: u32,
}

#[derive(Deserialize, Serialize)]
struct RawBonusUse {
    pub bonus: String,
    pub problem: u32,
}

#[derive(Deserialize, Serialize)]
struct RawSolutionState {
    pub dislikes: u64,
}
