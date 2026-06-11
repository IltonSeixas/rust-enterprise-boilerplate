use axum::{extract::State, http::header, response::IntoResponse};
use metrics_exporter_prometheus::PrometheusHandle;

pub async fn metrics(State(handle): State<PrometheusHandle>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        handle.render(),
    )
}
