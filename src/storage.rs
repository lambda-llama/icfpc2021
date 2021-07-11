use std::path::Path;

use crate::common::*;
use crate::problem::*;

lazy_static! {
    pub static ref PROBLEMS_PATH: &'static Path = {
        let path = Path::new("./problems");
        assert!(path.exists());
        path
    };
    pub static ref SOLUTIONS_PATH: &'static Path = {
        let path = Path::new("./solutions");
        assert!(path.exists());
        path
    };
}

pub fn get_problems_count() -> u32 {
    PROBLEMS_PATH.read_dir().unwrap().count() as u32
}

pub fn load_problem(id: u32) -> Result<Problem> {
    Problem::from_json(&std::fs::read(
        PROBLEMS_PATH.join(format!("{}.problem", id)),
    )?)
}

pub fn load_solution(id: u32) -> Result<Option<Solution>> {
    let path = SOLUTIONS_PATH.join(format!("{}.solution", id));
    let state_path = SOLUTIONS_PATH.join(format!("{}.state", id));
    if !path.exists() {
        return Ok(None);
    }
    let pose = Pose::from_json(&std::fs::read(path)?)?;
    if !state_path.exists() {
        warn!("No state file for {}.solution", id);
        return Ok(Some(Solution {
            id,
            pose,
            state: SolutionState::new(),
        }));
    }
    let state = SolutionState::from_json(&std::fs::read(state_path)?)?;
    Ok(Some(Solution { id, pose, state }))
}

pub fn load_custom_solution(path: &Path) -> Result<Pose> {
    Ok(Pose::from_json(&std::fs::read(path)?)?)
}

pub fn save_solution_state(solution: &Solution) -> Result<()> {
    Ok(std::fs::write(
        SOLUTIONS_PATH.join(format!("{}.state", solution.id)),
        solution.state.to_json()?,
    )?)
}
