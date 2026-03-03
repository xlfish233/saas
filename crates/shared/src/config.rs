//! Configuration management

use config::{Config as ConfigRs, Environment, File};
use serde::Deserialize;
use std::sync::OnceLock;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_environment")]
    pub environment: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,

    #[serde(default = "default_pool_size")]
    pub pool_size: u32,

    #[serde(default)]
    pub migration: crate::db::MigrationConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NatsConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub private_key_path: String,
    pub public_key_path: String,
    #[serde(default = "default_jwt_issuer")]
    pub issuer: String,
    #[serde(default = "default_jwt_audience")]
    pub audience: String,
    #[serde(default = "default_access_token_expiry")]
    pub access_token_expiry_seconds: i64,
    #[serde(default = "default_refresh_token_expiry")]
    pub refresh_token_expiry_seconds: i64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_environment() -> String {
    "local".to_string()
}
fn default_pool_size() -> u32 {
    10
}
fn default_jwt_issuer() -> String {
    "erp-saas".to_string()
}
fn default_jwt_audience() -> String {
    "erp-saas-api".to_string()
}
fn default_access_token_expiry() -> i64 {
    900
} // 15 minutes
fn default_refresh_token_expiry() -> i64 {
    604800
} // 7 days

impl AppConfig {
    pub fn load() -> Result<Self, crate::Error> {
        let config = ConfigRs::builder()
            // Load from file if exists
            .add_source(File::with_name("config").required(false))
            // Environment variables with __ separator
            .add_source(Environment::default().separator("__").try_parsing(true))
            .build()
            .map_err(|e| crate::Error::Config(e.to_string()))?;

        config
            .try_deserialize()
            .map_err(|e| crate::Error::Config(e.to_string()))
    }

    pub fn global() -> &'static Self {
        CONFIG.get_or_init(|| Self::load().expect("Failed to load configuration"))
    }

    pub fn is_production(&self) -> bool {
        self.server.environment == "production"
    }

    pub fn is_local(&self) -> bool {
        self.server.environment == "local"
    }
}

impl DatabaseConfig {
    pub fn to_connection_string(&self) -> String {
        self.url.clone()
    }
}
