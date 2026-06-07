use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    pub jwt_secret: String,
    #[serde(default = "default_access_ttl")]
    pub jwt_access_ttl_seconds: i64,
    #[serde(default = "default_refresh_ttl")]
    pub jwt_refresh_ttl_seconds: i64,
    pub redis_url: Option<String>,
    #[cfg(feature = "postgres")]
    pub database_url: String,
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: String,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_rate_limit_per_second")]
    pub rate_limit_per_second: u64,
    #[serde(default = "default_rate_limit_burst")]
    pub rate_limit_burst: u32,
}

fn default_host() -> String {
    "0.0.0.0".into()
}

fn default_port() -> u16 {
    8080
}

fn default_grpc_port() -> u16 {
    50051
}

fn default_access_ttl() -> i64 {
    900
}

fn default_refresh_ttl() -> i64 {
    604800
}

fn default_allowed_origins() -> String {
    "http://localhost:3000".into()
}

fn default_rate_limit_per_second() -> u64 {
    10
}

fn default_rate_limit_burst() -> u32 {
    20
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".into()
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();

        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }

    /// Parses the comma-separated `allowed_origins` value into a list of
    /// trimmed, non-empty origins for the CORS allow-list.
    pub fn allowed_origin_list(&self) -> Vec<String> {
        self.allowed_origins
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(String::from)
            .collect()
    }
}
