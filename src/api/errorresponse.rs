use std::fmt::Display;

use poem::{error::ResponseError, http::StatusCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub struct UpstreamError;

impl Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Upstream Error")
    }
}

impl ResponseError for UpstreamError {
    fn status(&self) -> poem::http::StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}
