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
        todo!()
    }

    fn center(&mut self, _f: &Figure, _v: usize) {
        todo!()
    }
}
