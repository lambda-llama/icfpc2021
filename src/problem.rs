use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Point {
    pub x: u64,
    pub y: u64,
}

#[derive(Debug)]
pub struct Figure {
    pub vertices: Vec<Point>,
    pub edges: Vec<(u64, u64)>,
}

#[derive(Debug)]
pub struct Problem {
    pub hole: Vec<Point>,
    pub figure: Figure,
    pub epsilon: u64,
}

impl Problem {
    pub fn from_json(data: &[u8]) -> serde_json::Result<Self> {
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
                edges: edges.into_iter().map(|e| (e[0], e[1])).collect(),
            },
            epsilon,
        })
    }
}

#[derive(Debug)]
pub struct Pose {
    pub vertices: Vec<Point>,
}

impl Pose {
    pub fn to_json(&self) -> serde_json::Result<String> {
        let pose = RawPose {
            vertices: self.vertices.iter().map(|p| vec![p.x, p.y]).collect()
        };
        serde_json::to_string(&pose)
    }
}

// Serialization helper types below

#[derive(Deserialize)]
struct RawFigure {
    pub vertices: Vec<Vec<u64>>,
    pub edges: Vec<Vec<u64>>,
}

#[derive(Deserialize)]
struct RawProblem {
    pub hole: Vec<Vec<u64>>,
    pub figure: RawFigure,
    pub epsilon: u64,
}

#[derive(Serialize)]
struct RawPose {
    pub vertices: Vec<Vec<u64>>,
}
