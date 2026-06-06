use std::sync::Arc;

use tokio::net::TcpListener;

mod application;
mod config;
mod domain;
mod infrastructure;
mod interfaces;

use application::use_cases::{
    ChangePassword, GetUser, LoginUser, RefreshTokenUseCase, RegisterUser, UpdateProfile,
};
use config::AppConfig;
use infrastructure::{
    persistence::InMemoryUserRepository,
    security::{Argon2Hasher, JwtTokenService},
    telemetry::{init_prometheus, init_tracing},
};
use interfaces::http::{
    build_router,
    handlers::{
        auth_handler::AuthState,
        user_handler::UserState,
    },
    middleware::AuthMiddlewareState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env().expect("failed to load configuration");

    init_tracing("rust-enterprise-boilerplate");
    let _prometheus = init_prometheus();

    let redis_url = cfg
        .redis_url
        .clone()
        .unwrap_or_else(|| "redis://127.0.0.1:6379".into());

    let redis_client = redis::Client::open(redis_url)?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client).await?;

    let user_repo: Arc<dyn domain::repositories::UserRepository> =
        Arc::new(InMemoryUserRepository::new()) as Arc<dyn domain::repositories::UserRepository>;
    let hasher: Arc<dyn application::ports::PasswordHasher> =
        Arc::new(Argon2Hasher::new()) as Arc<dyn application::ports::PasswordHasher>;
    let token_svc: Arc<dyn application::ports::TokenService> =
        Arc::new(JwtTokenService::new(
            &cfg.jwt_secret,
            cfg.jwt_access_ttl_seconds,
            cfg.jwt_refresh_ttl_seconds,
            redis_conn,
        )) as Arc<dyn application::ports::TokenService>;

    let auth_state = AuthState {
        register: Arc::new(RegisterUser::new(
            Arc::clone(&user_repo),
            Arc::clone(&hasher),
            Arc::clone(&token_svc),
        )),
        login: Arc::new(LoginUser::new(
            Arc::clone(&user_repo),
            Arc::clone(&hasher),
            Arc::clone(&token_svc),
        )),
        refresh: Arc::new(RefreshTokenUseCase::new(
            Arc::clone(&user_repo),
            Arc::clone(&token_svc),
        )),
    };

    let user_state = UserState {
        get_user: Arc::new(GetUser::new(Arc::clone(&user_repo))),
        update_profile: Arc::new(UpdateProfile::new(Arc::clone(&user_repo))),
        change_password: Arc::new(ChangePassword::new(
            Arc::clone(&user_repo),
            Arc::clone(&hasher),
        )),
    };

    let auth_mw_state = AuthMiddlewareState {
        token_svc: Arc::clone(&token_svc),
        user_repo: Arc::clone(&user_repo),
    };

    let router = build_router(auth_state, user_state, auth_mw_state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, "server listening");

    axum::serve(listener, router).await?;

    Ok(())
}
