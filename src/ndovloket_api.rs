//! This module gets the timetable file from http://data.ndovloket.nl
use std::{error::Error, time::Duration};

use thiserror::Error;

const TIMETABLE_URL: &str = "http://data.ndovloket.nl/ns/ns-latest.zip";
pub struct NDovLoket {}

#[derive(Error, Debug)]
pub enum NdovLoketError {
    #[error("HTTP request error: {0}")]
    Network(reqwest::Error),
}

impl NDovLoket {
    pub fn fetch_timetable() -> Result<Vec<u8>, Box<dyn Error>> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(NdovLoketError::Network)?;

        let request = client
            .get(TIMETABLE_URL)
            .build()
            .map_err(NdovLoketError::Network)?;

        Ok(client
            .execute(request)
            .map_err(NdovLoketError::Network)
            .map_err(Box::new)?
            .bytes()
            .map_err(NdovLoketError::Network)
            .map(|b| b.into())
            .map_err(Box::new)?)
    }
}
