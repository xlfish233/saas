//! Configuration

use config::{Config as ConfigRs, File, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
    pub jwt_private_key_path: String,
    pub jwt_public_key_path: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub environment: String,
    /// Comma-separated list of allowed CORS origins
    /// Example: "http://localhost:3000,https://app.example.com"
    pub cors_origins: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = ConfigRs::builder()
            .set_default("host", "0.0.0.0")?
            .set_default("port", 8080)?
            .set_default("environment", "local")?
            .add_source(Environment::default().separator("__"))
            .build()?;
        
        Ok(config.try_deserialize()?)
    }
}
