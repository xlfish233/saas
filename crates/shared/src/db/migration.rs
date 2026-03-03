//! Startup database migration and version gate utilities.

use anyhow::{anyhow, bail, Context};
use serde::Deserialize;
use sqlx::{migrate::Migrator, postgres::PgPoolOptions, query_scalar, PgPool};
use std::{env, path::PathBuf, time::Duration};
use tokio::time::sleep;

const EMBEDDED_LATEST_VERSION: &str = include_str!("../../../../migrations/LATEST_VERSION");

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MigrationRole {
    Owner,
    #[default]
    Verifier,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MigrationConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub role: MigrationRole,
    pub required_version: Option<i64>,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: u64,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            role: MigrationRole::default(),
            required_version: None,
            max_retries: default_max_retries(),
            base_delay_ms: default_base_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MigrationStatus {
    pub role: MigrationRole,
    pub current_version: i64,
    pub required_version: i64,
}

fn default_enabled() -> bool {
    true
}

fn default_max_retries() -> u32 {
    5
}

fn default_base_delay_ms() -> u64 {
    500
}

pub async fn connect_with_retry(
    database_url: &str,
    pool_size: u32,
    settings: &MigrationConfig,
) -> anyhow::Result<PgPool> {
    for attempt in 0..=settings.max_retries {
        match PgPoolOptions::new()
            .max_connections(pool_size)
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(err) if attempt < settings.max_retries => {
                let delay = backoff_delay(settings.base_delay_ms, attempt);
                tracing::warn!(
                    attempt,
                    max_retries = settings.max_retries,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "database connection failed, retrying"
                );
                sleep(delay).await;
            }
            Err(err) => {
                return Err(anyhow!(err)).context(format!(
                    "failed to connect to database after {} attempts",
                    settings.max_retries + 1
                ));
            }
        }
    }

    unreachable!("retry loop always returns before reaching this point");
}

pub async fn run_startup_migration_or_verify(
    pool: &PgPool,
    settings: &MigrationConfig,
) -> anyhow::Result<MigrationStatus> {
    let required_version = resolve_required_version(settings)?;

    if !settings.enabled {
        let current_version = fetch_current_db_version(pool).await.unwrap_or(0);
        tracing::warn!(
            role = ?settings.role,
            current_version,
            required_version,
            "database migration disabled by configuration"
        );

        return Ok(MigrationStatus {
            role: settings.role,
            current_version,
            required_version,
        });
    }

    match settings.role {
        MigrationRole::Owner => run_owner_migration(pool, settings, required_version).await,
        MigrationRole::Verifier => run_verifier_check(pool, required_version).await,
    }
}

pub async fn fetch_current_db_version(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let query =
        "SELECT COALESCE(MAX(version), 0)::BIGINT FROM _sqlx_migrations WHERE success = true";

    match query_scalar::<_, i64>(query).fetch_one(pool).await {
        Ok(version) => Ok(version),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => Ok(0),
        Err(err) => Err(err),
    }
}

async fn run_owner_migration(
    pool: &PgPool,
    settings: &MigrationConfig,
    required_version: i64,
) -> anyhow::Result<MigrationStatus> {
    let before_version = fetch_current_db_version(pool).await.unwrap_or(0);

    for attempt in 0..=settings.max_retries {
        match run_migrate_up_once(pool).await {
            Ok(()) => break,
            Err(err) if attempt < settings.max_retries => {
                let delay = backoff_delay(settings.base_delay_ms, attempt);
                tracing::warn!(
                    attempt,
                    max_retries = settings.max_retries,
                    delay_ms = delay.as_millis(),
                    error = %err,
                    "migration failed, retrying"
                );
                sleep(delay).await;
            }
            Err(err) => {
                return Err(err).context(format!(
                    "migration failed after {} attempts",
                    settings.max_retries + 1
                ));
            }
        }
    }

    let current_version = fetch_current_db_version(pool)
        .await
        .context("failed to fetch current database version after migration")?;

    if current_version < required_version {
        bail!(
            "database version too old after migrate up: current={}, required={}",
            current_version,
            required_version
        );
    }

    tracing::info!(
        role = "owner",
        before_version,
        current_version,
        required_version,
        "startup migration completed"
    );

    Ok(MigrationStatus {
        role: MigrationRole::Owner,
        current_version,
        required_version,
    })
}

async fn run_verifier_check(
    pool: &PgPool,
    required_version: i64,
) -> anyhow::Result<MigrationStatus> {
    let current_version = fetch_current_db_version(pool)
        .await
        .context("failed to fetch current database version")?;

    if current_version < required_version {
        bail!(
            "database version gate failed: current={}, required={}",
            current_version,
            required_version
        );
    }

    tracing::info!(
        role = "verifier",
        current_version,
        required_version,
        "database version gate passed"
    );

    Ok(MigrationStatus {
        role: MigrationRole::Verifier,
        current_version,
        required_version,
    })
}

async fn run_migrate_up_once(pool: &PgPool) -> anyhow::Result<()> {
    let migration_dir = resolve_migrations_dir()?;
    let migrator = Migrator::new(migration_dir.as_path())
        .await
        .with_context(|| format!("failed to load migrations from {}", migration_dir.display()))?;

    migrator
        .run(pool)
        .await
        .with_context(|| format!("failed to run migrations from {}", migration_dir.display()))
}

pub fn resolve_required_version(settings: &MigrationConfig) -> anyhow::Result<i64> {
    match settings.required_version {
        Some(version) if version >= 0 => Ok(version),
        Some(version) => bail!("required migration version must be >= 0, got {}", version),
        None => parse_version(EMBEDDED_LATEST_VERSION, "migrations/LATEST_VERSION"),
    }
}

pub fn resolve_migrations_dir() -> anyhow::Result<PathBuf> {
    if let Ok(path) = env::var("DATABASE__MIGRATION__DIR") {
        if !path.trim().is_empty() {
            let path = PathBuf::from(path);
            if path.is_dir() {
                return Ok(path);
            }
        }
    }

    if let Ok(path) = env::var("MIGRATION_DIR") {
        if !path.trim().is_empty() {
            let path = PathBuf::from(path);
            if path.is_dir() {
                return Ok(path);
            }
        }
    }

    let candidates = [
        PathBuf::from("migrations"),
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations")),
    ];

    for candidate in candidates {
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "cannot find migrations directory; set DATABASE__MIGRATION__DIR to an existing path"
    ))
}

fn parse_version(raw: &str, source: &str) -> anyhow::Result<i64> {
    raw.trim().parse::<i64>().with_context(|| {
        format!(
            "{} must contain a single integer migration version, got {:?}",
            source,
            raw.trim()
        )
    })
}

fn backoff_delay(base_delay_ms: u64, attempt: u32) -> Duration {
    let shift = attempt.min(16);
    let factor = 1_u64.checked_shl(shift).unwrap_or(u64::MAX);
    Duration::from_millis(base_delay_ms.saturating_mul(factor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_verifier() {
        let settings = MigrationConfig::default();
        assert_eq!(settings.role, MigrationRole::Verifier);
        assert!(settings.enabled);
        assert_eq!(settings.max_retries, 5);
        assert_eq!(settings.base_delay_ms, 500);
    }

    #[test]
    fn parse_version_accepts_newline() {
        let version = parse_version("20240301000000\n", "test").expect("version should parse");
        assert_eq!(version, 20240301000000);
    }

    #[test]
    fn resolve_required_version_prefers_override() {
        let settings = MigrationConfig {
            required_version: Some(123),
            ..MigrationConfig::default()
        };

        let version = resolve_required_version(&settings).expect("override should be used");
        assert_eq!(version, 123);
    }

    #[test]
    fn backoff_delay_grows_exponentially() {
        assert_eq!(backoff_delay(500, 0), Duration::from_millis(500));
        assert_eq!(backoff_delay(500, 1), Duration::from_millis(1000));
        assert_eq!(backoff_delay(500, 2), Duration::from_millis(2000));
    }
}
