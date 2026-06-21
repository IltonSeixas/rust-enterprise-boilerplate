use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct HealthState {
    pub redis: redis::aio::ConnectionManager,
    #[cfg(feature = "postgres")]
    pub database: sqlx::PgPool,
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

pub async fn ready(State(state): State<HealthState>) -> impl IntoResponse {
    let mut checks = serde_json::Map::new();
    let mut is_ready = true;

    let mut redis_conn = state.redis;
    match redis::cmd("PING")
        .query_async::<String>(&mut redis_conn)
        .await
    {
        Ok(_) => {
            checks.insert("redis".into(), Value::String("ok".into()));
        }
        Err(_) => {
            checks.insert("redis".into(), Value::String("error".into()));
            is_ready = false;
        }
    }

    #[cfg(feature = "postgres")]
    {
        match sqlx::query("SELECT 1").execute(&state.database).await {
            Ok(_) => {
                checks.insert("postgres".into(), Value::String("ok".into()));
            }
            Err(_) => {
                checks.insert("postgres".into(), Value::String("error".into()));
                is_ready = false;
            }
        }
    }

    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let status_text = if is_ready { "ready" } else { "not_ready" };

    (
        status,
        Json(json!({ "status": status_text, "checks": checks })),
    )
}
