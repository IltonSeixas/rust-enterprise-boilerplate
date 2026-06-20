use std::time::Duration;

use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use metrics_exporter_prometheus::PrometheusHandle;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::PeerIpKeyExtractor, GovernorLayer,
};
use tower_http::trace::TraceLayer;

use crate::interfaces::http::{
    handlers::{
        auth_handler::{login, refresh, register, AuthState},
        health_handler::{health, ready},
        metrics_handler::metrics,
        user_handler::{
            change_password, change_role, get_me, get_user, list_users, update_me, UserState,
        },
    },
    middleware::{build_cors_layer, require_auth, security_headers, AuthMiddlewareState},
};

pub struct RouterConfig {
    pub allowed_origins: Vec<String>,
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u32,
}

pub fn build_router(
    auth_state: AuthState,
    user_state: UserState,
    auth_mw_state: AuthMiddlewareState,
    metrics_handle: PrometheusHandle,
    router_cfg: RouterConfig,
) -> Router {
    let rate_limit_per_second = router_cfg.rate_limit_per_second.max(1) as u32;
    let governor_cfg = GovernorConfigBuilder::default()
        .key_extractor(PeerIpKeyExtractor)
        .period(Duration::from_secs(1) / rate_limit_per_second)
        .burst_size(router_cfg.rate_limit_burst)
        .finish()
        .expect("invalid rate limiter configuration");
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics).with_state(metrics_handle))
        .nest(
            "/v1/auth",
            Router::new()
                .route("/register", post(register))
                .route("/login", post(login))
                .route("/refresh", post(refresh))
                .with_state(auth_state),
        );

    let protected_routes = Router::new()
        .nest(
            "/v1/users",
            Router::new()
                .route("/me", get(get_me).put(update_me))
                .route("/me/password", put(change_password))
                .route("/", get(list_users))
                .route("/:id", get(get_user))
                .route("/:id/role", put(change_role))
                .with_state(user_state),
        )
        .layer(middleware::from_fn_with_state(auth_mw_state, require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security_headers))
        .layer(build_cors_layer(&router_cfg.allowed_origins))
        .layer(GovernorLayer::new(governor_cfg))
}
