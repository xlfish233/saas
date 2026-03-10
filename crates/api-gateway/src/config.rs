//! Configuration

use config::{Config as ConfigRs, Environment};
use serde::Deserialize;
use shared::db::MigrationConfig;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub nats: NatsConfig,
    pub jwt: JwtConfig,
    #[serde(default)]
    pub auth_service: AuthServiceConfig,
    #[serde(default)]
    pub cors: CorsConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[allow(dead_code)]
pub struct AuthServiceConfig {
    #[serde(default = "default_auth_service_url")]
    pub url: String,
}

fn default_auth_service_url() -> String {
    "http://127.0.0.1:8081".to_string()
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_environment")]
    pub environment: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    #[serde(default)]
    pub migration: MigrationConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct NatsConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct JwtConfig {
    pub private_key_path: String,
    pub public_key_path: String,
    #[serde(default = "default_jwt_issuer")]
    pub issuer: String,
    #[serde(default = "default_jwt_audience")]
    pub audience: String,
    #[serde(default = "default_access_token_expiry")]
    pub access_token_expiry_seconds: u64,
    #[serde(default = "default_refresh_token_expiry")]
    pub refresh_token_expiry_seconds: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct CorsConfig {
    #[serde(default)]
    pub origins: Option<String>,
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
fn default_access_token_expiry() -> u64 {
    900
}
fn default_refresh_token_expiry() -> u64 {
    604800
}

impl Config {
    #[allow(dead_code)]
    pub fn from_env() -> anyhow::Result<Self> {
        let config = ConfigRs::builder()
            .add_source(Environment::default().separator("__").try_parsing(true))
            .build()?;

        Ok(config.try_deserialize()?)
    }

    #[allow(dead_code)]
    pub fn database_url(&self) -> &str {
        &self.database.url
    }
    #[allow(dead_code)]
    pub fn redis_url(&self) -> &str {
        &self.redis.url
    }
    #[allow(dead_code)]
    pub fn nats_url(&self) -> &str {
        &self.nats.url
    }
    pub fn cors_origins(&self) -> Option<&String> {
        self.cors.origins.as_ref()
    }
    pub fn auth_service_url(&self) -> &str {
        &self.auth_service.url
    }
}
