use anyhow::{anyhow, bail, Context};
use config::{Config as ConfigRs, Environment, File};
use serde::Deserialize;
use shared::db::MigrationConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationCliConfig {
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
    #[serde(default)]
    pub migration: MigrationConfig,
}

fn default_pool_size() -> u32 {
    10
}

impl MigrationCliConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = ConfigRs::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::default().separator("__").try_parsing(true))
            .build()
            .context("failed to load db-migrator configuration sources")?;

        Self::from_config(config)
    }

    fn from_config(config: ConfigRs) -> anyhow::Result<Self> {
        let parsed: Self = config.try_deserialize().map_err(|e| {
            anyhow!(
                "failed to deserialize db-migrator config: {e}. \\
                 db-migrator only needs DATABASE__* variables and requires DATABASE__URL"
            )
        })?;

        if parsed.database.url.trim().is_empty() {
            bail!("DATABASE__URL must be non-empty");
        }

        Ok(parsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_with_database_only_config() {
        let config = ConfigRs::builder()
            .set_override("database.url", "postgres://dev:dev@localhost:5432/dev")
            .expect("set database url")
            .build()
            .expect("build config");

        let parsed = MigrationCliConfig::from_config(config).expect("parse config");
        assert_eq!(parsed.database.url, "postgres://dev:dev@localhost:5432/dev");
        assert_eq!(parsed.database.pool_size, 10);
    }

    #[test]
    fn fails_without_database_url() {
        let config = ConfigRs::builder()
            .set_override("database.pool_size", 8)
            .expect("set pool size")
            .build()
            .expect("build config");

        let err = MigrationCliConfig::from_config(config).expect_err("must fail");
        let msg = err.to_string();
        assert!(msg.contains("DATABASE__URL") || msg.contains("missing field `url`"));
    }

    #[test]
    fn parses_nested_migration_fields() {
        let config = ConfigRs::builder()
            .set_override("database.url", "postgres://dev:dev@localhost:5432/dev")
            .expect("set database url")
            .set_override("database.migration.max_retries", 9)
            .expect("set retries")
            .set_override("database.migration.base_delay_ms", 1234)
            .expect("set delay")
            .set_override("database.migration.enabled", false)
            .expect("set enabled")
            .build()
            .expect("build config");

        let parsed = MigrationCliConfig::from_config(config).expect("parse config");
        assert_eq!(parsed.database.migration.max_retries, 9);
        assert_eq!(parsed.database.migration.base_delay_ms, 1234);
        assert!(!parsed.database.migration.enabled);
    }
}
