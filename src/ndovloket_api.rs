//! This module gets the timetable file from <http://data.ndovloket.nl>
use std::time::Duration;

use thiserror::Error;

const TIMETABLE_URL: &str = "http://data.ndovloket.nl/ns/ns-latest.zip";
pub struct NDovLoket {}

#[derive(Error, Debug)]
pub enum NdovLoketError {
    #[error("HTTP request error: {0}")]
    Network(reqwest::Error),
}

impl NDovLoket {
    pub async fn fetch_timetable() -> Result<Vec<u8>, NdovLoketError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(NdovLoketError::Network)?;

        let request = client
            .get(TIMETABLE_URL)
            .build()
            .map_err(NdovLoketError::Network)?;

        client
            .execute(request)
            .await
            .map_err(NdovLoketError::Network)?
            .bytes()
            .await
            .map_err(NdovLoketError::Network)
            .map(std::convert::Into::into)
    }
}
