use chrono::Duration;
use poem::{handler, http::header, Response};

use std::sync::Arc;

use poem::web::Data;

use crate::api::datarepo::DataRepo;

#[handler]
pub fn company_endpoint(data: Data<&Arc<DataRepo>>, _req: String) -> Response {
    let data = serde_json::to_vec(&data.0.companies()).unwrap();

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(data)
}
