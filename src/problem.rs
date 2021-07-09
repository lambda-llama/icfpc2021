use geo::prelude::Contains;
use ordered_float::NotNan;
use serde_derive::{Deserialize, Serialize};
use geo::relate::Relate;

use crate::common::*;

pub type Point = geo::Point<i64>;

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

    pub fn distance(p: Point, q: Point) -> f64 {
        ((p.x() - q.x()) as f64).powi(2) + ((p.y() - q.y()) as f64).powi(2)
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
        (
            (1.0f64 - self.epsilon) * len_default,
            (1.0f64 + self.epsilon) * len_default,
        )
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
        let mut border: Vec<geo::Point<f64>> = hole.clone().into_iter().map(|p| geo::Point::new(p.x() as f64, p.y() as f64)).collect();
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
            hole.into_iter().map(|p| Point::new(p[0], p[1])).collect(),
            Figure::new(
                vertices
                    .into_iter()
                    .map(|p| Point::new(p[0], p[1]))
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
                    .map(|&p| NotNan::new(Figure::distance(p, v)).unwrap())
                    .min()
                    .unwrap()
                    .into_inner()
            })
            .sum();
        sum.trunc() as u64
    }

    pub fn validate(&self, pose: &Pose) -> bool {
        for p in &pose.vertices {
            let fp = geo::Point::new(p.x() as f64, p.y() as f64);
            let relation = self.poly.relate(&fp);
            if ! (relation.is_within() || relation.is_intersects()) {
                return false
            }
        }
        // TODO(acherepanov): Take into account that segments could be out of the polygon as well.
        true
    }
}

#[derive(Debug)]
pub struct Pose {
    pub vertices: Vec<Point>,
}

impl Pose {
    pub fn from_json(data: &[u8]) -> Result<Self> {
        let RawPose { vertices } = serde_json::from_slice(data)?;
        Ok(Pose {
            vertices: vertices
                .into_iter()
                .map(|p| Point::new(p[0], p[1]))
                .collect(),
        })
    }

    pub fn to_json(&self) -> Result<String> {
        let pose = RawPose {
            vertices: self.vertices.iter().map(|p| vec![p.x(), p.y()]).collect(),
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
