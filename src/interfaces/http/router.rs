use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use metrics_exporter_prometheus::PrometheusHandle;
use tower_governor::GovernorLayer;
use tower_http::trace::TraceLayer;

use crate::interfaces::{
    http::{
        handlers::{
            auth_handler::{login, refresh, register, AuthState},
            health_handler::{health, ready, HealthState},
            metrics_handler::metrics,
            user_handler::{
                change_password, change_role, get_me, get_user, list_users, update_me, UserState,
            },
        },
        middleware::{build_cors_layer, require_auth, security_headers, AuthMiddlewareState},
    },
    rate_limit::build_governor_config,
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
    health_state: HealthState,
    metrics_handle: PrometheusHandle,
    router_cfg: RouterConfig,
) -> anyhow::Result<Router> {
    let governor_cfg = build_governor_config(
        router_cfg.rate_limit_per_second,
        router_cfg.rate_limit_burst,
    )?;
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready).with_state(health_state.clone()))
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

    Ok(Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security_headers))
        .layer(build_cors_layer(&router_cfg.allowed_origins))
        .layer(GovernorLayer::new(governor_cfg)))
}
