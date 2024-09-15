use poem::{
    handler,
    http::{header, StatusCode},
    IntoResponse, Response,
};

use std::sync::Arc;

use poem::web::Data;

use crate::api::{datarepo::DataRepo, IntoAPIObject};

#[handler]
pub fn active_rides_endpoint(data: Data<&Arc<DataRepo>>, _req: String) -> Response {
    let now = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Amsterdam);

    let rides: Vec<_> = data
        .as_ref()
        .rides_active_at_time(&now.naive_local().time(), &now.date_naive())
        .iter()
        .map(|r| r.as_api_object())
        .collect();

    let data = serde_json::to_vec(&rides);

    match data {
        Ok(json) => Response::builder()
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .body(json),
        Err(e) => {
            eprintln!("{}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
