use crate::api::datarepo::DataRepo;

use super::RoutePlannerResponse;

use ns_api::TripAdviceArguments;

use poem::{handler, http::header, Response};

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
) -> Response {
    query.validate(&station_allow_list);

    println!("Request from: {} to: {}", query.from, query.to);

    let ns_data = ns_api
        .find_path(&TripAdviceArguments {
            from: &query.from,
            to: &query.to,
            via: None,
        })
        .await
        .unwrap();

    let out = RoutePlannerResponse::new(&ns_data, &datarepo);

    Response::builder()
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(serde_json::to_vec(&out).unwrap())
}
