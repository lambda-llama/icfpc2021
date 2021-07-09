use geo::algorithm::contains::Contains;
use geo::algorithm::euclidean_distance::EuclideanDistance;
use geo::relate::Relate;
use ordered_float::NotNan;
use serde_derive::{Deserialize, Serialize};

use crate::common::*;

pub type Point = geo::Coordinate<i64>;

#[derive(Debug)]
pub struct Figure {
    pub vertices: Vec<Point>,
    pub edges: Vec<(usize, usize)>,
    pub vertex_edges: Vec<Vec<(usize, usize)>>,
    pub epsilon: f64,
}

impl Figure {
    pub fn new(vertices: Vec<Point>, edges: Vec<(usize, usize)>, epsilon: f64) -> Self {
        let mut vertex_edges = vec![Vec::new(); vertices.len()];
        for (i, e) in edges.iter().enumerate() {
            vertex_edges[e.0].push((i, e.1));
            vertex_edges[e.1].push((i, e.0));
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
        let e = self.edges[idx];
        let p = pose.vertices[e.0];
        let q = pose.vertices[e.1];
        Self::distance_squared(p, q)
    }

    pub fn edge_len2_default(&self, idx: usize) -> f64 {
        let e = self.edges[idx];
        let p = self.vertices[e.0];
        let q = self.vertices[e.1];
        Self::distance_squared(p, q)
    }

    pub fn edge_len2_bounds(&self, idx: usize) -> (f64, f64) {
        let weight_default = self.edge_len2_default(idx);
        (
            (1.0f64 - self.epsilon) * weight_default,
            (1.0f64 + self.epsilon) * weight_default,
        )
    }

    // TODO: bool => enum (ok, close to bad, bad)
    pub fn test_edge_len2(&self, idx: usize, pose: &Pose) -> bool {
        let (min, max) = self.edge_len2_bounds(idx);
        let len = self.edge_len2(idx, pose);
        if len < min || len > max {
            false
        } else {
            true
        }
    }
}

#[derive(Debug)]
pub struct Problem {
    pub hole: Vec<Point>,
    pub poly: geo::Polygon<f64>,
    pub figure: Figure,
}

impl Problem {
    pub fn new(hole: Vec<Point>, figure: Figure) -> Self {
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
        }
    }

    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawProblem {
            hole,
            figure: RawFigure { vertices, edges },
            epsilon,
        } = serde_json::from_slice(data)?;
        Ok(Problem::new(
            hole.into_iter()
                .map(|p| Point { x: p[0], y: p[1] })
                .collect(),
            Figure::new(
                vertices
                    .into_iter()
                    .map(|p| Point { x: p[0], y: p[1] })
                    .collect(),
                edges
                    .into_iter()
                    .map(|e| (e[0] as usize, e[1] as usize))
                    .collect(),
                epsilon as f64 / 1_000_000.0f64,
            ),
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
        let to_fp = |p: Point| geo::Point::new(p.x as f64, p.y as f64);
        for &p in &pose.vertices {
            let relation = self.poly.relate(&to_fp(p));
            if !(relation.is_within() || relation.is_intersects()) {
                return false;
            }
        }
        // 2 - edges are inside
        for (u, v) in &self.figure.edges {
            let s = geo::LineString::from(vec![to_fp(pose.vertices[*u]), to_fp(pose.vertices[*v])]);
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
        let p = geo::Point::new(point.x as f64, point.y as f64);
        if self.poly.contains(&p) {
            return 0.0;
        }
        return self.poly.euclidean_distance(&p);
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

#[derive(Clone, Debug, Default)]
pub struct Pose {
    pub vertices: Vec<Point>,
}

impl Pose {
    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawPose { vertices } = serde_json::from_slice(data)?;
        Ok(Pose {
            vertices: vertices
                .into_iter()
                .map(|p| Point { x: p[0], y: p[1] })
                .collect(),
        })
    }

    pub fn to_json(&self) -> Result<String> {
        let pose = RawPose {
            vertices: self.vertices.iter().map(|p| vec![p.x, p.y]).collect(),
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
}

#[derive(Deserialize, Serialize)]
struct RawPose {
    pub vertices: Vec<Vec<i64>>,
}
