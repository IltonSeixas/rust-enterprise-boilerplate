pub mod auth_service;
pub mod error;
pub mod interceptor;
pub mod rate_limit;
pub mod user_service;

pub mod proto {
    tonic::include_proto!("boilerplate.v1");
}

pub use auth_service::AuthGrpcService;
pub use proto::auth_service_server::AuthServiceServer;
pub use proto::user_service_server::UserServiceServer;
pub use rate_limit::ConnectInfoBridgeLayer;
pub use user_service::UserGrpcService;
