use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub jwt_secret: String,
    #[serde(default = "default_access_ttl")]
    pub jwt_access_ttl_seconds: i64,
    #[serde(default = "default_refresh_ttl")]
    pub jwt_refresh_ttl_seconds: i64,
    pub redis_url: Option<String>,
    #[cfg(feature = "postgres")]
    pub database_url: String,
}

fn default_host() -> String {
    "0.0.0.0".into()
}

fn default_port() -> u16 {
    8080
}

fn default_access_ttl() -> i64 {
    900
}

fn default_refresh_ttl() -> i64 {
    604800
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();

        config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize()
    }
}
