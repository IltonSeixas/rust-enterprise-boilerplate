use std::sync::Arc;

use tokio::net::TcpListener;
use tonic::transport::Server as GrpcServer;

mod application;
mod config;
mod domain;
mod infrastructure;
mod interfaces;

use application::use_cases::{
    ChangePassword, ChangeUserRole, GetUser, ListUsers, LoginUser, RefreshTokenUseCase,
    RegisterUser, UpdateProfile,
};
use config::AppConfig;
#[cfg(not(feature = "postgres"))]
use infrastructure::audit::InMemoryAuditLog;
#[cfg(feature = "postgres")]
use infrastructure::audit::PostgresAuditLog;
#[cfg(not(feature = "postgres"))]
use infrastructure::persistence::InMemoryUserRepository;
#[cfg(feature = "postgres")]
use infrastructure::persistence::PostgresUserRepository;
use infrastructure::{
    resilience::CircuitBreaker,
    security::{Argon2Hasher, JwtTokenService},
    telemetry::{init_prometheus, init_tracing},
};
use interfaces::{
    grpc::{AuthGrpcService, AuthServiceServer, UserGrpcService, UserServiceServer},
    http::{
        build_router,
        handlers::{auth_handler::AuthState, health_handler::HealthState, user_handler::UserState},
        middleware::AuthMiddlewareState,
        RouterConfig,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env().expect("failed to load configuration");

    let tracer_provider = init_tracing("rust-enterprise-boilerplate", &cfg.otlp_endpoint)?;
    let metrics_handle = init_prometheus();

    let redis_url = cfg
        .redis_url
        .clone()
        .unwrap_or_else(|| "redis://127.0.0.1:6379".into());

    let redis_client = redis::Client::open(redis_url)?;
    let redis_conn = tokio::time::timeout(
        std::time::Duration::from_millis(cfg.redis_connect_timeout_ms),
        redis::aio::ConnectionManager::new(redis_client),
    )
    .await
    .map_err(|_| anyhow::anyhow!("timed out connecting to redis"))??;

    #[cfg(feature = "postgres")]
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(cfg.db_pool_max_connections)
        .min_connections(cfg.db_pool_min_connections)
        .acquire_timeout(std::time::Duration::from_millis(
            cfg.db_pool_connect_timeout_ms,
        ))
        .idle_timeout(std::time::Duration::from_millis(
            cfg.db_pool_idle_timeout_ms,
        ))
        .max_lifetime(std::time::Duration::from_millis(
            cfg.db_pool_max_lifetime_ms,
        ))
        .connect(&cfg.database_url)
        .await?;
    #[cfg(feature = "postgres")]
    infrastructure::persistence::migrations::run(&pool).await?;

    #[cfg(feature = "postgres")]
    let user_repo: Arc<dyn domain::repositories::UserRepository> =
        Arc::new(PostgresUserRepository::new(pool.clone()))
            as Arc<dyn domain::repositories::UserRepository>;
    #[cfg(feature = "postgres")]
    let audit: Arc<dyn application::ports::AuditPort> =
        Arc::new(PostgresAuditLog::new(pool.clone())) as Arc<dyn application::ports::AuditPort>;

    #[cfg(not(feature = "postgres"))]
    let (user_repo, audit): (
        Arc<dyn domain::repositories::UserRepository>,
        Arc<dyn application::ports::AuditPort>,
    ) = (
        Arc::new(InMemoryUserRepository::new()) as Arc<dyn domain::repositories::UserRepository>,
        Arc::new(InMemoryAuditLog::new()) as Arc<dyn application::ports::AuditPort>,
    );
    let hasher: Arc<dyn application::ports::PasswordHasher> =
        Arc::new(Argon2Hasher::new()) as Arc<dyn application::ports::PasswordHasher>;
    let jwt_private_key =
        std::fs::read(&cfg.jwt_private_key_path).expect("failed to read JWT_PRIVATE_KEY_PATH");
    let jwt_public_key =
        std::fs::read(&cfg.jwt_public_key_path).expect("failed to read JWT_PUBLIC_KEY_PATH");
    let resilience_cfg = cfg.resilience();
    let redis_breaker = CircuitBreaker::new(
        resilience_cfg.failure_threshold,
        resilience_cfg.reset_timeout_ms,
    );
    let token_svc: Arc<dyn application::ports::TokenService> = Arc::new(
        JwtTokenService::new(
            &jwt_private_key,
            &jwt_public_key,
            cfg.jwt_access_ttl_seconds,
            cfg.jwt_refresh_ttl_seconds,
            redis_conn.clone(),
            redis_breaker,
            resilience_cfg.retry_policy(),
            cfg.redis_command_timeout_ms,
        )
        .expect("failed to load Ed25519 JWT keys"),
    )
        as Arc<dyn application::ports::TokenService>;

    let auth_state = AuthState {
        register: Arc::new(RegisterUser::new(
            Arc::clone(&user_repo),
            Arc::clone(&hasher),
            Arc::clone(&token_svc),
            Arc::clone(&audit),
        )),
        login: Arc::new(LoginUser::new(
            Arc::clone(&user_repo),
            Arc::clone(&hasher),
            Arc::clone(&token_svc),
            Arc::clone(&audit),
        )),
        refresh: Arc::new(RefreshTokenUseCase::new(
            Arc::clone(&user_repo),
            Arc::clone(&token_svc),
            Arc::clone(&audit),
        )),
    };

    let user_state = UserState {
        get_user: Arc::new(GetUser::new(Arc::clone(&user_repo))),
        list_users: Arc::new(ListUsers::new(Arc::clone(&user_repo))),
        update_profile: Arc::new(UpdateProfile::new(Arc::clone(&user_repo))),
        change_password: Arc::new(ChangePassword::new(
            Arc::clone(&user_repo),
            Arc::clone(&hasher),
            Arc::clone(&audit),
        )),
        change_role: Arc::new(ChangeUserRole::new(
            Arc::clone(&user_repo),
            Arc::clone(&audit),
        )),
    };

    let auth_mw_state = AuthMiddlewareState {
        token_svc: Arc::clone(&token_svc),
        user_repo: Arc::clone(&user_repo),
    };

    let health_state = HealthState {
        redis: redis_conn,
        #[cfg(feature = "postgres")]
        database: pool,
    };

    let auth_grpc = AuthGrpcService {
        register: Arc::clone(&auth_state.register),
        login: Arc::clone(&auth_state.login),
        refresh: Arc::clone(&auth_state.refresh),
    };
    let user_grpc = UserGrpcService {
        get_user: Arc::clone(&user_state.get_user),
        list_users: Arc::clone(&user_state.list_users),
        update_profile: Arc::clone(&user_state.update_profile),
        change_password: Arc::clone(&user_state.change_password),
        change_role: Arc::clone(&user_state.change_role),
        token_svc: Arc::clone(&token_svc),
        user_repo: Arc::clone(&user_repo),
    };

    let router_cfg = RouterConfig {
        allowed_origins: cfg.allowed_origin_list(),
        rate_limit_per_second: cfg.rate_limit_per_second,
        rate_limit_burst: cfg.rate_limit_burst,
    };
    let router = build_router(
        auth_state,
        user_state,
        auth_mw_state,
        health_state,
        metrics_handle,
        router_cfg,
    );

    let http_addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = TcpListener::bind(&http_addr).await?;
    tracing::info!(address = %http_addr, "http server listening");

    let grpc_addr: std::net::SocketAddr = format!("{}:{}", cfg.host, cfg.grpc_port).parse()?;
    tracing::info!(address = %grpc_addr, "grpc server listening");

    let http_server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );
    let grpc_server = GrpcServer::builder()
        .add_service(AuthServiceServer::new(auth_grpc))
        .add_service(UserServiceServer::new(user_grpc))
        .serve(grpc_addr);

    let result = tokio::try_join!(
        async { http_server.await.map_err(anyhow::Error::from) },
        async { grpc_server.await.map_err(anyhow::Error::from) },
    );

    tracer_provider
        .shutdown()
        .map_err(|e| anyhow::anyhow!("failed to shut down tracer provider: {e}"))?;

    result?;

    Ok(())
}
