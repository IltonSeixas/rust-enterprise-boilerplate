use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();

    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=63072000; includeSubDomains"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    // This is a JSON/gRPC API with no HTML rendering, so the strictest
    // policy applies: nothing is permitted to load.
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    res
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request, middleware, routing::get, Router};
    use tower::ServiceExt;

    use super::security_headers;

    fn app() -> Router {
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(middleware::from_fn(security_headers))
    }

    #[tokio::test]
    async fn injects_expected_security_headers() {
        let response = app()
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let headers = response.headers();
        assert_eq!(headers["x-content-type-options"], "nosniff");
        assert_eq!(headers["x-frame-options"], "DENY");
        assert_eq!(
            headers["strict-transport-security"],
            "max-age=63072000; includeSubDomains"
        );
        assert_eq!(
            headers["referrer-policy"],
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            headers["content-security-policy"],
            "default-src 'none'; frame-ancestors 'none'"
        );
    }
}
