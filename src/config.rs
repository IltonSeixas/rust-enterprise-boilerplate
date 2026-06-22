use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    pub jwt_private_key_path: String,
    pub jwt_public_key_path: String,
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
    #[serde(default = "default_cb_failure_threshold")]
    pub circuit_breaker_failure_threshold: u32,
    #[serde(default = "default_cb_reset_timeout_ms")]
    pub circuit_breaker_reset_timeout_ms: u64,
    #[serde(default = "default_retry_max_attempts")]
    pub retry_max_attempts: u32,
    #[serde(default = "default_retry_initial_backoff_ms")]
    pub retry_initial_backoff_ms: u64,
    #[serde(default = "default_retry_backoff_multiplier")]
    pub retry_backoff_multiplier: u32,
    #[serde(default = "default_db_pool_max_connections")]
    pub db_pool_max_connections: u32,
    #[serde(default = "default_db_pool_min_connections")]
    pub db_pool_min_connections: u32,
    #[serde(default = "default_db_pool_connect_timeout_ms")]
    pub db_pool_connect_timeout_ms: u64,
    #[serde(default = "default_db_pool_idle_timeout_ms")]
    pub db_pool_idle_timeout_ms: u64,
    #[serde(default = "default_db_pool_max_lifetime_ms")]
    pub db_pool_max_lifetime_ms: u64,
    #[serde(default = "default_redis_connect_timeout_ms")]
    pub redis_connect_timeout_ms: u64,
    #[serde(default = "default_redis_command_timeout_ms")]
    pub redis_command_timeout_ms: u64,
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

fn default_cb_failure_threshold() -> u32 {
    5
}

fn default_cb_reset_timeout_ms() -> u64 {
    30_000
}

fn default_retry_max_attempts() -> u32 {
    3
}

fn default_retry_initial_backoff_ms() -> u64 {
    50
}

fn default_retry_backoff_multiplier() -> u32 {
    2
}

fn default_db_pool_max_connections() -> u32 {
    10
}

fn default_db_pool_min_connections() -> u32 {
    2
}

fn default_db_pool_connect_timeout_ms() -> u64 {
    30_000
}

fn default_db_pool_idle_timeout_ms() -> u64 {
    600_000
}

fn default_db_pool_max_lifetime_ms() -> u64 {
    1_800_000
}

fn default_redis_connect_timeout_ms() -> u64 {
    2_000
}

fn default_redis_command_timeout_ms() -> u64 {
    2_000
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

    pub fn resilience(&self) -> crate::infrastructure::resilience::ResilienceConfig {
        crate::infrastructure::resilience::ResilienceConfig {
            failure_threshold: self.circuit_breaker_failure_threshold,
            reset_timeout_ms: self.circuit_breaker_reset_timeout_ms,
            retry_max_attempts: self.retry_max_attempts,
            retry_initial_backoff_ms: self.retry_initial_backoff_ms,
            retry_backoff_multiplier: self.retry_backoff_multiplier,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_required_env() {
        std::env::set_var("JWT_PRIVATE_KEY_PATH", "/tmp/private.pem");
        std::env::set_var("JWT_PUBLIC_KEY_PATH", "/tmp/public.pem");
        #[cfg(feature = "postgres")]
        std::env::set_var("DATABASE_URL", "postgres://user:pass@localhost/db");
    }

    fn clear_pool_env() {
        for key in [
            "DB_POOL_MAX_CONNECTIONS",
            "DB_POOL_MIN_CONNECTIONS",
            "DB_POOL_CONNECT_TIMEOUT_MS",
            "DB_POOL_IDLE_TIMEOUT_MS",
            "DB_POOL_MAX_LIFETIME_MS",
            "REDIS_CONNECT_TIMEOUT_MS",
            "REDIS_COMMAND_TIMEOUT_MS",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn pool_and_redis_timeout_fields_fall_back_to_documented_defaults() {
        set_required_env();
        clear_pool_env();

        let cfg = AppConfig::from_env().expect("config should load with only required fields set");

        assert_eq!(cfg.db_pool_max_connections, 10);
        assert_eq!(cfg.db_pool_min_connections, 2);
        assert_eq!(cfg.db_pool_connect_timeout_ms, 30_000);
        assert_eq!(cfg.db_pool_idle_timeout_ms, 600_000);
        assert_eq!(cfg.db_pool_max_lifetime_ms, 1_800_000);
        assert_eq!(cfg.redis_connect_timeout_ms, 2_000);
        assert_eq!(cfg.redis_command_timeout_ms, 2_000);
    }

    #[test]
    fn pool_and_redis_timeout_fields_read_from_env() {
        set_required_env();
        std::env::set_var("DB_POOL_MAX_CONNECTIONS", "25");
        std::env::set_var("DB_POOL_MIN_CONNECTIONS", "5");
        std::env::set_var("DB_POOL_CONNECT_TIMEOUT_MS", "15000");
        std::env::set_var("DB_POOL_IDLE_TIMEOUT_MS", "300000");
        std::env::set_var("DB_POOL_MAX_LIFETIME_MS", "900000");
        std::env::set_var("REDIS_CONNECT_TIMEOUT_MS", "1500");
        std::env::set_var("REDIS_COMMAND_TIMEOUT_MS", "1500");

        let cfg = AppConfig::from_env().expect("config should load with overrides set");

        assert_eq!(cfg.db_pool_max_connections, 25);
        assert_eq!(cfg.db_pool_min_connections, 5);
        assert_eq!(cfg.db_pool_connect_timeout_ms, 15_000);
        assert_eq!(cfg.db_pool_idle_timeout_ms, 300_000);
        assert_eq!(cfg.db_pool_max_lifetime_ms, 900_000);
        assert_eq!(cfg.redis_connect_timeout_ms, 1_500);
        assert_eq!(cfg.redis_command_timeout_ms, 1_500);

        clear_pool_env();
    }
}
