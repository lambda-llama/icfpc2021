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
    Problem::from_json(
        id,
        &std::fs::read(PROBLEMS_PATH.join(format!("{}.problem", id)))?,
    )
}

pub fn load_solution(id: u32) -> Result<Option<Solution>> {
    let path = SOLUTIONS_PATH.join(format!("{}.solution", id));
    let state_path = SOLUTIONS_PATH.join(format!("{}.meta", id));
    if !path.exists() || !state_path.exists() {
        return Ok(None);
    }
    Ok(Some(Solution {
        id,
        pose: Pose::from_json(&std::fs::read(path)?)?,
        state: SolutionState::from_json(&std::fs::read(state_path)?)?,
        server_state: load_server_state(id)?,
    }))
}

pub fn save_solution(solution: &Solution, subfolder: Option<&str>) -> Result<()> {
    let solutions_path = match subfolder {
        Some(s) => SOLUTIONS_PATH.join(s),
        None => SOLUTIONS_PATH.to_owned(),
    };
    std::fs::write(
        &solutions_path.join(format!("{}.solution", solution.id)),
        solution.pose.to_json()?,
    )?;
    std::fs::write(
        solutions_path.join(format!("{}.meta", solution.id)),
        solution.state.to_json()?,
    )?;
    Ok(())
}

pub fn load_custom_solution(path: &Path) -> Result<Pose> {
    Ok(Pose::from_json(&std::fs::read(path)?)?)
}

pub fn load_server_state(id: u32) -> Result<ServerState> {
    let server_state_path = SOLUTIONS_PATH.join(format!("{}.state", id));
    if server_state_path.exists() {
        ServerState::from_json(&std::fs::read(
            SOLUTIONS_PATH.join(format!("{}.state", id)),
        )?)
    } else {
        Ok(ServerState::new())
    }
}

pub fn save_server_state(id: u32, state: &ServerState) -> Result<()> {
    Ok(std::fs::write(
        SOLUTIONS_PATH.join(format!("{}.state", id)),
        state.to_json()?,
    )?)
}
