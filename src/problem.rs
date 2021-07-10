use geo::algorithm::contains::Contains;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::relate::Relate;
use ordered_float::NotNan;
use serde_derive::{Deserialize, Serialize};

use crate::common::*;

pub type Point = geo::Coordinate<i64>;

#[derive(Debug)]
pub struct Edge {
    pub v0: usize,
    pub v1: usize,
    pub len2: f64,
}

#[derive(Debug)]
pub struct Figure {
    pub vertices: Vec<Point>,
    pub edges: Vec<Edge>,
    pub vertex_edges: Vec<Vec<(usize, usize)>>,
    pub epsilon: f64,
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

    pub fn edge_len2_diff(&self, idx: usize, pose: &Pose) -> f64 {
        (self.edge_len2(idx, pose) - self.edges[idx].len2).abs()
    }

    // TODO: bool => enum (ok, close to bad, bad)
    pub fn test_edge_len2(&self, idx: usize, pose: &Pose) -> bool {
        let diff = self.edge_len2_diff(idx, pose);
        let allowed = self.epsilon * self.edges[idx].len2;
        if diff <= allowed {
            true
        } else {
            false
        }
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

#[derive(Debug)]
pub struct BonusUnlock {
    position: Point,
    bonus: BonusType,
    problem: u32,
}

#[derive(Debug)]
pub struct Problem {
    pub hole: Vec<Point>,
    pub poly: geo::Polygon<f64>,
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

        Self {
            hole: hole,
            poly: geo::Polygon::new(geo::LineString::from(border), vec![]),
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

    pub fn validate(&self, pose: &Pose) -> bool {
        // 1 - vertices are inside
        for &p in &pose.vertices {
            let relation = self.poly.relate(&p.convert());
            if !(relation.is_within() || relation.is_intersects()) {
                return false;
            }
        }
        // 2 - edges are inside
        for e in &self.figure.edges {
            let s = geo::LineString::from(vec![
                pose.vertices[e.v0].convert(),
                pose.vertices[e.v1].convert(),
            ]);
            let relation = self.poly.relate(&s);
            if !relation.is_contains() {
                return false;
            }
        }
        // 3 - edges are of correct length
        for idx in 0..self.figure.edges.len() {
            if !self.figure.test_edge_len2(idx, pose) {
                return false;
            }
        }
        true
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
        // TODO: Count weighted intersections with edges of polygon.
        if self.poly.contains(&edge) {
            return 0.0;
        }
        return 1.0;
    }

    pub fn bounding_box(&self) -> (Point, Point) {
        let mut min_p = Point {
            x: i64::MAX,
            y: i64::MAX,
        };
        let mut max_p = Point { x: 0, y: 0 };
        let it = self.hole.iter().chain(self.hole.iter());
        for p in it {
            min_p.x = std::cmp::min(min_p.x, p.x);
            max_p.x = std::cmp::max(max_p.x, p.x);
            min_p.y = std::cmp::min(min_p.y, p.y);
            max_p.y = std::cmp::max(max_p.y, p.y);
        }
        return (min_p, max_p);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BonusUse {
    bonus: BonusType,
    problem: u32,
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
