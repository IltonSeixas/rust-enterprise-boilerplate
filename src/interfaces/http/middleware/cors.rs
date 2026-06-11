use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

const ALLOWED_METHODS: [Method; 5] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::OPTIONS,
];

/// Builds a CORS layer restricted to an explicit origin allow-list.
///
/// The wildcard `*` is never honored: an `Origin` header that does not match
/// the allow-list simply receives no CORS headers.
pub fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods(ALLOWED_METHODS)
        .allow_headers([
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
        ])
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt;

    use super::build_cors_layer;

    fn app() -> Router {
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(build_cors_layer(&["http://localhost:3000".to_string()]))
    }

    #[tokio::test]
    async fn reflects_listed_origin() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .header("origin", "http://localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "http://localhost:3000"
        );
    }

    #[tokio::test]
    async fn omits_header_for_unlisted_origin() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/ping")
                    .header("origin", "http://evil.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(!response
            .headers()
            .contains_key("access-control-allow-origin"));
    }
}
