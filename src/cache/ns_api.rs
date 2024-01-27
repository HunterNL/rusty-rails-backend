use reqwest::blocking::Client;

const API_HOST: &str = "https://gateway.apiportal.ns.nl/";
const ROUTE_PATH: &str = "Spoorkaart-API/api/v1/spoorkaart";
const STATION_PATH: &str = "reisinformatie-api/api/v2/stations";

pub struct NsApi {
    key: String,
    client: Client,
}

impl NsApi {
    fn get_file(&self, path: &str) -> Result<bytes::Bytes, String> {
        let request = self
            .client
            .get(String::new() + API_HOST + path)
            .header("Ocp-Apim-Subscription-Key", &self.key)
            .build()
            .map_err(|e| format!("Error constructing HTTP request: {e}"))?;

        self.client
            .execute(request)
            .map_err(|e| format!("Error making HTTP request: {e}"))?
            .bytes()
            .map_err(|e| format!("Error getting bytes: {e}"))
    }

    pub fn stations(&self) -> Result<bytes::Bytes, String> {
        self.get_file(STATION_PATH)
    }

    pub fn routes(&self) -> Result<bytes::Bytes, String> {
        self.get_file(ROUTE_PATH)
    }
}

impl NsApi {
    pub fn new(key: String) -> Self {
        Self {
            key,
            client: Client::new(),
        }
    }
}
