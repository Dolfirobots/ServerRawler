use axum::{Json, extract::Query, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use crate::webapi::stats;

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct DeleteParams {
    pub id: i32,
}

#[derive(Deserialize)]
pub struct ScanParams {
    pub generate: bool,
    pub amount: Option<i32>,
    pub cidr: Option<String>,
}

pub async fn get_stats() -> impl IntoResponse {
    Json(stats::fetch_stats().await)
}

pub async fn search_servers(Query(params): Query<SearchParams>) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

pub async fn delete_server(Query(params): Query<DeleteParams>) -> impl IntoResponse {
    format!("Deleting server with ID: {}", params.id)
}

pub async fn start_scan(
    Query(params): Query<ScanParams>,
    body: String
) -> impl IntoResponse {
    if params.generate {
        (StatusCode::ACCEPTED, "Generation scan started".to_string())
    } else {
        let ip_count = body.lines().count();
        (StatusCode::ACCEPTED, format!("Scanning {} provided IPs", ip_count))
    }
}