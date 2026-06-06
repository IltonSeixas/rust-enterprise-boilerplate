use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::interfaces::http::{
    handlers::{
        auth_handler::{login, refresh, register, AuthState},
        health_handler::{health, ready},
        user_handler::{change_password, get_me, get_user, update_me, UserState},
    },
    middleware::{require_auth, AuthMiddlewareState},
};

pub fn build_router(
    auth_state: AuthState,
    user_state: UserState,
    auth_mw_state: AuthMiddlewareState,
) -> Router {
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
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
                .route("/:id", get(get_user))
                .with_state(user_state),
        )
        .layer(middleware::from_fn_with_state(auth_mw_state, require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
