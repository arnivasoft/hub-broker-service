use anyhow::Result;
use common::{DatabaseConfig, RedisConfig, SecurityConfig, ServerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub security: SecurityConfig,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let server = ServerConfig {
            host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()?,
            max_connections: std::env::var("MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10000".to_string())
                .parse()?,
            heartbeat_interval_secs: std::env::var("HEARTBEAT_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
            message_timeout_secs: std::env::var("MESSAGE_TIMEOUT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()?,
        };

        let database = DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()?,
            min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()?,
            connect_timeout_secs: std::env::var("DB_CONNECT_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
        };

        let redis = RedisConfig {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            pool_size: std::env::var("REDIS_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
        };

        let security = SecurityConfig {
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            jwt_expiry_secs: std::env::var("JWT_EXPIRY")
                .unwrap_or_else(|_| "900".to_string())
                .parse()?,
            require_tls: std::env::var("REQUIRE_TLS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()?,
            rate_limit_per_sec: std::env::var("RATE_LIMIT")
                .unwrap_or_else(|_| "100".to_string())
                .parse()?,
        };

        Ok(Config {
            server,
            database,
            redis,
            security,
        })
    }
}
