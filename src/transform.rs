use crate::problem::*;

// A separate trait for `Pose` transformations to have a clearer API
// for these algorithms
pub trait Transform {
    // Fold (mirror) a component selected by `vcomp` over a line defined by `v1` and `v2`
    fn fold(&mut self, f: &Figure, v1: usize, v2: usize, vcomp: usize);

    // "Center" a vertex by minimizing the sum of errors of its edges
    fn center(&mut self, f: &Figure, v: usize, search_region: (Point, Point));

    // Rotate a vertex around the pivot
    fn rotate(&mut self, f: &Figure, v: usize, v_pivot: usize, angle: f64);
}

impl Transform for Pose {
    fn fold(&mut self, _f: &Figure, _v1: usize, _v2: usize, _vcomp: usize) {
        let n = _f.vertices.len();
        let mut components =  vec![-1; n];
        components[_v1] = 0;
        components[_v2] = 0;

        fn dfs(u: usize, vertex_edges: &Vec<Vec<(usize, usize)>>, components: &mut Vec<i32>, cid: usize) {
            components[u] = cid as i32;
            for &(_, v) in &vertex_edges[u] {
                if components[v] == -1 {
                    dfs(v, vertex_edges, components, cid);
                }
            }
        }
        let mut cid = 1;
        for u in 0 .. n {
            if components[u] == -1 {
                dfs(u, &_f.vertex_edges, &mut components, cid);
                cid += 1;
            }
        }
        let p1 = Figure::to_float_point(self.vertices[_v1]);
        let p2 = Figure::to_float_point(self.vertices[_v2]);
        let a = p2 - p1;
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

    fn center(&mut self, _f: &Figure, _v: usize, search_region: (Point, Point)) {
        let (mn, mx) = search_region;
        let p = self.vertices[_v];
        let loss = |p: Point| -> f64 {
            let mut res = 0.0f64;
            for &(idx, w) in &_f.vertex_edges[_v] {
                let e =&_f.edges[idx];
                res += (Figure::distance_squared(self.vertices[w], p) / e.len2 - 1.0f64).abs();
            }
            res
        };
        let mut q = p;
        let mut best_loss = loss(p);

        for x in mn.x..mx.x+1 {
            for y in mn.y..mx.y + 1 {
                let t = Point{x, y};
                let loss_t =loss(t);
                if loss_t < best_loss {
                    best_loss = loss_t;
                    q = t;
                }
            }
        }
        self.vertices[_v] = q;
    }

    fn rotate(&mut self, _f: &Figure, _v: usize, _v_pivot: usize, _angle: f64) {
        todo!()
    }
}
