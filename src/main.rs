use std::sync::Arc;

use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;

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
use interfaces::{
    grpc::{AuthGrpcService, AuthServiceServer, UserGrpcService, UserServiceServer},
    http::{
        build_router,
        handlers::{
            auth_handler::AuthState,
            user_handler::UserState,
        },
        middleware::AuthMiddlewareState,
    },
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

    let auth_grpc = AuthGrpcService {
        register: Arc::clone(&auth_state.register),
        login: Arc::clone(&auth_state.login),
        refresh: Arc::clone(&auth_state.refresh),
    };
    let user_grpc = UserGrpcService {
        get_user: Arc::clone(&user_state.get_user),
        update_profile: Arc::clone(&user_state.update_profile),
        change_password: Arc::clone(&user_state.change_password),
        token_svc: Arc::clone(&token_svc),
        user_repo: Arc::clone(&user_repo),
    };

    let router = build_router(auth_state, user_state, auth_mw_state);

    let http_addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&http_addr).await?;
    tracing::info!(address = %http_addr, "http server listening");

    let grpc_addr: std::net::SocketAddr = format!("{}:{}", cfg.host, cfg.grpc_port).parse()?;
    tracing::info!(address = %grpc_addr, "grpc server listening");

    let http_server = axum::serve(listener, router);
    let grpc_server = GrpcServer::builder()
        .add_service(AuthServiceServer::new(auth_grpc))
        .add_service(UserServiceServer::new(user_grpc))
        .serve(grpc_addr);

    tokio::try_join!(
        async { http_server.await.map_err(anyhow::Error::from) },
        async { grpc_server.await.map_err(anyhow::Error::from) },
    )?;

    Ok(())
}
