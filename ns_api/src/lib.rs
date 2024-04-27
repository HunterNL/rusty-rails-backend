use reqwest::{Client, IntoUrl, Request, RequestBuilder};
use thiserror::Error;

const API_HOST: &str = "https://gateway.apiportal.ns.nl/";
const ROUTE_PATH: &str = "Spoorkaart-API/api/v1/spoorkaart";
const STATION_PATH: &str = "reisinformatie-api/api/v2/stations";
const TRIP_PATH: &str = "reisinformatie-api/api/v3/trips";

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request error: {0}")]
    Network(reqwest::Error),
    #[error("Error parsing response data: {0}")]
    Parsing(serde_json::Error),
}

pub struct NsApi {
    key: String,
    client: Client,
}

pub struct TripAdviceArguments<'a, 'b, 'c> {
    pub from: &'a str,
    pub to: &'b str,
    pub via: Option<&'c str>,
}

mod response_data {
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    struct Product {
        number: String,
        categoryCode: String,
    }

    #[derive(Deserialize, Debug)]
    #[allow(non_snake_case)]
    struct Location {
        stationCode: String,
    }

    #[derive(Deserialize, Debug)]
    struct Leg {
        name: String,
        origin: Location,
        destination: Location,
        product: Product,
    }

    #[derive(Deserialize, Debug)]
    struct Trip {
        legs: Vec<Leg>,
    }

    #[derive(Deserialize, Debug)]
    pub struct Response {
        trips: Vec<Trip>,
    }
}

pub use response_data::Response;

impl NsApi {
    pub fn new(key: String) -> Self {
        Self {
            key,
            client: Client::new(),
        }
    }

    pub async fn fetch_stations(&self) -> Result<Vec<u8>, ApiError> {
        let url = API_HOST.to_owned() + STATION_PATH;
        let rb = self.start_request(url);
        let req = rb.build().map_err(ApiError::Network)?;

        self.fetch_as_bytes(req).await.map(std::convert::Into::into)
    }

    pub async fn fetch_routes(&self) -> Result<Vec<u8>, ApiError> {
        let url = API_HOST.to_owned() + ROUTE_PATH;
        let rb = self.start_request(url);
        let req = rb.build().map_err(ApiError::Network)?;

        self.fetch_as_bytes(req).await.map(std::convert::Into::into)
    }

    async fn fetch_as_bytes(&self, request: Request) -> Result<bytes::Bytes, ApiError> {
        self.client
            .execute(request)
            .await
            .map_err(ApiError::Network)?
            .bytes()
            .await
            .map_err(ApiError::Network)
    }

    async fn fetch_as_string(&self, request: Request) -> Result<String, ApiError> {
        self.client
            .execute(request)
            .await
            .map_err(ApiError::Network)?
            .text()
            .await
            .map_err(ApiError::Network)
    }

    fn start_request(&self, url: impl IntoUrl) -> RequestBuilder {
        self.client
            .get(url)
            .header("Ocp-Apim-Subscription-Key", &self.key)
    }

    pub async fn find_path(
        &self,
        args: &TripAdviceArguments<'_, '_, '_>,
    ) -> Result<response_data::Response, ApiError> {
        let url = API_HOST.to_owned() + TRIP_PATH;

        println!("{url}");
        let mut request = self
            .start_request(url)
            .query(&[("fromStation", args.from), ("toStation", args.to)]);

        if let Some(via) = args.via {
            request = request.query(&[("viaStation", via)]);
        }

        let request = request.build().map_err(ApiError::Network)?;

        let res = self
            .client
            .execute(request)
            .await
            .map_err(ApiError::Network)?;

        let byteslice = res.bytes().await.map_err(ApiError::Network)?;

        // let byteslice = TESTDATA;

        let response_data: response_data::Response =
            serde_json::from_slice(&byteslice).map_err(ApiError::Parsing)?;

        Ok(response_data)
    }
}
