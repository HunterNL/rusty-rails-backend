use crate::api::datarepo::DataRepo;

use super::RoutePlannerResponse;

use ns_api::TripAdviceArguments;

use poem::{handler, http::header, web::Json, Response};

use super::PathfindingArguments;

use ns_api::NsApi;

use std::sync::Arc;

use poem::web::Data;

#[handler]
pub async fn route_finding_endpoint(
    ns_api: Data<&Arc<NsApi>>,
    datarepo: Data<&Arc<DataRepo>>,
    query: poem::web::Query<PathfindingArguments>,
) -> Json<RoutePlannerResponse> {
    query.validate();

    let ns_data = ns_api
        .find_path(&TripAdviceArguments {
            from: &query.from,
            to: &query.to,
            via: None,
        })
        .await
        .unwrap();

    Json(RoutePlannerResponse::new(&ns_data, &datarepo))

    // Response::builder()
    //     .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
    //     .body(serde_json::to_vec(&response_data).unwrap())
}
