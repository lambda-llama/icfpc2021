use lazy_static;
use reqwest::{self, blocking::Client};

use crate::{common::*, problem::Pose};

pub struct Session {
    token: String,
    client: Client,
}

lazy_static! {
    pub static ref SESSION: Session = {
        let token = std::env::var("API_TOKEN").expect("Set the API_TOKEN environment variable");
        Session::new(token).expect("Failed to create a session")
    };
}

impl Session {
    pub fn new(token: String) -> Result<Self> {
        let client = Client::new();
        client
            .get("https://poses.live/api/hello")
            .bearer_auth(&token)
            .send()?
            .error_for_status()?;
        Ok(Session { token, client })
    }

    pub fn download_problem(&self, id: u64, path: &str) -> Result<()> {
        let resp = self
            .client
            .get(format!("https://poses.live/api/problems/{}", id))
            .bearer_auth(&self.token)
            .send()?
            .error_for_status()?;
        std::fs::write(path, resp.text()?)?;
        Ok(())
    }

    pub fn upload_solution(&self, id: u64, pose: &Pose) -> Result<()> {
        let data = pose
            .to_json()?
            .as_bytes()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let _ = self
            .client
            .post(format!("https://poses.live/api/problems/{}/solutions", id))
            .bearer_auth(&self.token)
            .body(data)
            .send()?
            .error_for_status()?;
        Ok(())
    }
}
