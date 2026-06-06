use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub async fn ready() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ready" })))
}
