use geomath::prelude::coordinates::Polar;

use crate::{common::PointConversion, problem::*};

// A separate trait for `Pose` transformations to have a clearer API
// for these algorithms
pub trait Transform {
    // Fold (mirror) a component selected by `vcomp` over a line defined by `v1` and `v2`
    fn fold(&mut self, f: &Figure, v1: usize, v2: usize, vcomp: usize);

    // Pulls all adjacent vertices closer (to the legal length)
    fn pull(&mut self, f: &Figure, v: usize);

    // "Center" a vertex by minimizing the sum of errors of its edges
    fn center(&mut self, f: &Figure, v: usize, search_region: (Point, Point));

    // Rotate a vertex around the pivot
    fn rotate(&mut self, v: usize, pivot: Point, angle_rad: f64);

    // Flip the point horizontally inside the region
    fn flip_h(&mut self, v: usize, region: (Point, Point));

    // Flip the point vertically inside the region
    fn flip_v(&mut self, v: usize, region: (Point, Point));
}

impl Transform for Pose {
    fn fold(&mut self, _f: &Figure, _v1: usize, _v2: usize, _vcomp: usize) {
        let n = _f.vertices.len();
        let mut components = vec![-1; n];

        let p1 = Figure::to_float_point(self.vertices[_v1]);
        let p2 = Figure::to_float_point(self.vertices[_v2]);
        let a = p2 - p1;
        for (u, p) in self.vertices.iter_mut().enumerate() {
            if p1.cross_prod(p2, p.convert()) == 0.0f64 {
                components[u] = 0;
            }
        }

        fn dfs(
            u: usize,
            vertex_edges: &Vec<Vec<(usize, usize)>>,
            components: &mut Vec<i32>,
            cid: usize,
        ) {
            components[u] = cid as i32;
            for &(_, v) in &vertex_edges[u] {
                if components[v] == -1 {
                    dfs(v, vertex_edges, components, cid);
                }
            }
        }
        let mut cid = 1;
        for u in 0..n {
            if components[u] == -1 {
                dfs(u, &_f.vertex_edges, &mut components, cid);
                cid += 1;
            }
        }

        for (u, p) in self.vertices.iter_mut().enumerate() {
            if components[u] != components[_vcomp] {
                continue;
            }
            let b = Figure::to_float_point(*p) - p1;
            let q = p1 + a * a.dot(b) / (a.dot(a)) * 2f64 - b;
            p.x = q.x().round() as i64;
            p.y = q.y().round() as i64;
        }
    }

    fn pull(&mut self, f: &Figure, v: usize) {
        let p0 = self.vertices[v];
        for &(e, v) in &f.vertex_edges[v] {
            if f.test_edge_len2(e, self) != EdgeTestResult::Ok {
                let p = self.vertices[v];
                let mut vec =
                    geomath::vector::Vector2::new((p.x - p0.x) as f64, (p.y - p0.y) as f64);
                vec.set_rho(f.edges[e].len2.sqrt());
                dbg!(vec.rho());
                self.vertices[v] = Point {
                    x: p0.x + vec.x as i64,
                    y: p0.y + vec.y as i64,
                }
            }
        }
    }

    fn center(&mut self, f: &Figure, v: usize, search_region: (Point, Point)) {
        let (mn, mx) = search_region;
        let p = self.vertices[v];
        let loss = |p: Point| -> f64 {
            let mut res = 0.0f64;
            for &(idx, w) in &f.vertex_edges[v] {
                let e = &f.edges[idx];
                res += (Figure::distance_squared(self.vertices[w], p) / e.len2 - 1.0f64).abs();
            }
            res
        };
        let mut q = p;
        let mut best_loss = loss(p);

        for x in mn.x..mx.x + 1 {
            for y in mn.y..mx.y + 1 {
                let t = Point { x, y };
                let loss_t = loss(t);
                if loss_t < best_loss {
                    best_loss = loss_t;
                    q = t;
                }
            }
        }
        self.vertices[v] = q;
    }

    fn rotate(&mut self, v: usize, pivot: Point, angle_rad: f64) {
        let p = self.vertices[v];
        let mut vec = geomath::vector::Vector2::new((p.x - pivot.x) as f64, (p.y - pivot.y) as f64);
        vec.set_phi(vec.phi() + angle_rad);
        self.vertices[v] = Point {
            x: pivot.x + vec.x as i64,
            y: pivot.y + vec.y as i64,
        };
    }

    fn flip_h(&mut self, v: usize, (min, max): (Point, Point)) {
        let p = self.vertices[v];
        self.vertices[v] = Point {
            x: min.x + (max.x - p.x),
            y: p.y,
        }
    }

    fn flip_v(&mut self, v: usize, (min, max): (Point, Point)) {
        let p = self.vertices[v];
        self.vertices[v] = Point {
            x: p.x,
            y: min.y + (max.y - p.y),
        }
    }
}
