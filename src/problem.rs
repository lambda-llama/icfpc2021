use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Figure {
    pub vertices: Vec<Vec<u64>>, // TODO: Vec<Point>
    pub edges: Vec<Vec<u64>>, // TODO: Vec<Edge>
}

#[derive(Debug, Deserialize)]
pub struct Problem {
    pub hole: Vec<Vec<u64>>, // TODO: Vec<Point>
    pub figure: Figure,
    pub epsilon: u64,
}

#[derive(Debug, Serialize)]
pub struct Pose {
    pub vertices: Vec<Vec<u64>> // TODO: Vec<Point>
}
