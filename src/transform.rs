use crate::problem::*;

// A separate trait for `Pose` transformations to have a clearer API
// for these algorithms
pub trait Transform {
    // Fold (mirror) a component selected by `vcomp` over a line defined by `v1` and `v2`
    fn fold(&mut self, f: &Figure, v1: usize, v2: usize, vcomp: usize);

    // "Center" a vertex by minimizing the sum of errors of its edges
    fn center(&mut self, f: &Figure, v: usize);
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
            let q = p1 + a * a.dot(b) / (a.dot(a)) - b;
            p.x = q.x().round() as i64;
            p.y = q.y().round() as i64;
        }
    }

    fn center(&mut self, _f: &Figure, _v: usize) {
        todo!()
    }
}
