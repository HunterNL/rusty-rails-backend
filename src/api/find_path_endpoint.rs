use crate::api::{datarepo::DataRepo, errorresponse::UpstreamError};

use super::RoutePlannerResponse;

use ns_api::TripAdviceArguments;

use poem::{handler, http::header, IntoResponse, Response, Result};

use super::PathfindingArguments;

use ns_api::NsApi;

use std::{collections::HashSet, sync::Arc};

use poem::web::Data;

#[handler]
pub async fn route_finding_endpoint(
    ns_api: Data<&Arc<NsApi>>,
    datarepo: Data<&Arc<DataRepo>>,
    query: poem::web::Query<PathfindingArguments>,
    station_allow_list: Data<&Arc<HashSet<Box<str>>>>,
) -> Result<Response> {
    query.validate(&station_allow_list);

    println!("Request from: {} to: {}", query.from, query.to);

    let ns_data = ns_api
        .find_path(&TripAdviceArguments {
            from: &query.from,
            to: &query.to,
            via: None,
        })
        .await
        .map_err(|e| eprint!("{:?}", e))
        .map_err(|_| UpstreamError {})?;

    let out = RoutePlannerResponse::new(&ns_data, &datarepo);
    let body = serde_json::to_vec(&out)
        .map_err(|e| eprint!("{:?}", e))
        .map_err(|_| UpstreamError {})?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(body)
        .into_response())
}
