use lazy_static;
use reqwest::{self, blocking::Client};

use crate::common::*;

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

    // Create a separate upload for `Pose` if needed
    pub fn upload_solution(&self, id: u64, path: &str) -> Result<()> {
        let data = std::fs::read(path)?;
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
