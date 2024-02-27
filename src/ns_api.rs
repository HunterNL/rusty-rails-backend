use reqwest::blocking::Client;
use thiserror::Error;

const API_HOST: &str = "https://gateway.apiportal.ns.nl/";
const ROUTE_PATH: &str = "Spoorkaart-API/api/v1/spoorkaart";
const STATION_PATH: &str = "reisinformatie-api/api/v2/stations";

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request error: {0}")]
    Network(reqwest::Error),
}

pub struct NsApi {
    key: String,
    client: Client,
}

impl NsApi {
    pub fn new(key: String) -> Self {
        Self {
            key,
            client: Client::new(),
        }
    }

    pub fn fetch_stations(&self) -> Result<Vec<u8>, ApiError> {
        self.fetch_as_bytes(STATION_PATH).map(|b| b.into())
    }

    pub fn fetch_routes(&self) -> Result<Vec<u8>, ApiError> {
        self.fetch_as_bytes(ROUTE_PATH).map(|b| b.into())
    }

    fn fetch_as_bytes(&self, path: &str) -> Result<bytes::Bytes, ApiError> {
        let request = self
            .client
            .get(String::new() + API_HOST + path)
            .header("Ocp-Apim-Subscription-Key", &self.key)
            .build()
            .map_err(ApiError::Network)?;

        self.client
            .execute(request)
            .map_err(ApiError::Network)?
            .bytes()
            .map_err(ApiError::Network)
    }
}
