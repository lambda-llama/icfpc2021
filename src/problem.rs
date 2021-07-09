use serde_derive::{Deserialize, Serialize};

use crate::common::*;

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug)]
pub struct Figure {
    pub vertices: Vec<Point>,
    pub edges: Vec<(usize, usize)>,
    pub epsilon: f64,
}

impl Figure {
    fn distance(p: Point, q: Point) -> f64 {
        ((p.x - q.x) as f64).powi(2) + ((p.y - q.y) as f64).powi(2)
    }

    pub fn edge_len(&self, idx: usize, pose: &Pose) -> f64 {
        let e = self.edges[idx];
        let p = pose.vertices[e.0];
        let q = pose.vertices[e.1];
        Self::distance(p, q)
    }

    pub fn edge_len_default(&self, idx: usize) -> f64 {
        let e = self.edges[idx];
        let p = self.vertices[e.0];
        let q = self.vertices[e.1];
        Self::distance(p, q)
    }

    pub fn edge_len_bounds(&self, idx: usize) -> (f64, f64) {
        let len_default = self.edge_len_default(idx);
        ((1.0f64 - self.epsilon) * len_default, (1.0f64 + self.epsilon) * len_default)
    }
}

#[derive(Debug)]
pub struct Problem {
    pub hole: Vec<Point>,
    pub figure: Figure,
}

impl Problem {
    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawProblem {
            hole,
            figure: RawFigure { vertices, edges },
            epsilon,
        } = serde_json::from_slice(&data)?;
        Ok(Problem {
            hole: hole
                .into_iter()
                .map(|p| Point { x: p[0], y: p[1] })
                .collect(),
            figure: Figure {
                vertices: vertices
                    .into_iter()
                    .map(|p| Point { x: p[0], y: p[1] })
                    .collect(),
                edges: edges.into_iter().map(|e| (e[0] as usize, e[1] as usize)).collect(),
                epsilon: epsilon as f64 / 1_000_000.0f64,
            },
        })
    }
}

#[derive(Debug)]
pub struct Pose {
    pub vertices: Vec<Point>,
}

impl Pose {
    pub fn to_json(&self) -> Result<String> {
        let pose = RawPose {
            vertices: self.vertices.iter().map(|p| vec![p.x, p.y]).collect()
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

#[derive(Serialize)]
struct RawPose {
    pub vertices: Vec<Vec<i64>>,
}
