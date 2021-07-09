use lazy_static;
use reqwest;
use serde_derive::Deserialize;

use crate::common::*;

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
}

lazy_static! {
    static ref CREDENTIALS: Credentials = {
        let data = std::fs::read(".credentials").expect("Failed to find the credentials file");
        serde_json::from_slice(&data).expect("Failed to parse the credentials")
    };
}

pub struct Session {
    client: reqwest::blocking::Client,
}

impl Session {
    pub fn new() -> Result<Self> {
        let form = reqwest::blocking::multipart::Form::new()
            .text("login.email", &CREDENTIALS.email)
            .text("login.password", &CREDENTIALS.password);
        let client = reqwest::blocking::ClientBuilder::new()
            .cookie_store(true)
            .build()?;
        client
            .post("https://poses.live/login?")
            .multipart(form)
            .send()?
            .error_for_status()?;
        Ok(Session { client })
    }

    pub fn download_problem(id: u64, path: &str) -> Result<()> {
        let resp = reqwest::blocking::get(format!("https://poses.live/problems/{}/download", id))?
            .error_for_status()?;
        std::fs::write(path, resp.text()?)?;
        Ok(())
    }

    // Create a separate upload for `Pose` if needed
    pub fn upload_solution(&self, id: u64, path: &str) -> Result<()> {
        let form = reqwest::blocking::multipart::Form::new().file("solution.body", path)?;
        let _ = self
            .client
            .post(format!("https://poses.live/problems/{}/solutions", id))
            .multipart(form)
            .send()?
            .error_for_status()?;
        Ok(())
    }
}
