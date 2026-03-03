use anyhow::{bail, Context};
use shared::db::{
    connect_with_retry, fetch_current_db_version, resolve_migrations_dir, resolve_required_version,
    run_startup_migration_or_verify, MigrationRole,
};
use sqlx::migrate::{MigrateDatabase, Migrator};
use sqlx::Postgres;

mod config;

use config::MigrationCliConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Up,
    Rollback,
    Reset,
    Verify,
    Version,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    shared::telemetry::init_tracing("db-migrator");

    let command = parse_command(std::env::args().skip(1))?;
    let config = MigrationCliConfig::from_env()?;
    let mut settings = config.database.migration.clone();

    match command {
        Command::Up => settings.role = MigrationRole::Owner,
        Command::Rollback => settings.role = MigrationRole::Owner,
        Command::Reset => settings.role = MigrationRole::Owner,
        Command::Verify => settings.role = MigrationRole::Verifier,
        Command::Version => {}
    }

    match command {
        Command::Up => {
            let pool =
                connect_with_retry(&config.database.url, config.database.pool_size, &settings)
                    .await
                    .context("failed to connect to database")?;
            let status = run_startup_migration_or_verify(&pool, &settings).await?;
            tracing::info!(
                role = ?status.role,
                current_version = status.current_version,
                required_version = status.required_version,
                "migrate up completed"
            );
        }
        Command::Rollback => {
            let pool =
                connect_with_retry(&config.database.url, config.database.pool_size, &settings)
                    .await
                    .context("failed to connect to database")?;
            rollback_latest(&pool).await?;
        }
        Command::Reset => {
            reset_database_and_migrate(&config, &settings).await?;
        }
        Command::Verify => {
            let pool =
                connect_with_retry(&config.database.url, config.database.pool_size, &settings)
                    .await
                    .context("failed to connect to database")?;
            let status = run_startup_migration_or_verify(&pool, &settings).await?;
            tracing::info!(
                role = ?status.role,
                current_version = status.current_version,
                required_version = status.required_version,
                "database version verify completed"
            );
        }
        Command::Version => {
            let pool =
                connect_with_retry(&config.database.url, config.database.pool_size, &settings)
                    .await
                    .context("failed to connect to database")?;
            let current = fetch_current_db_version(&pool).await?;
            let required = resolve_required_version(&settings)?;
            println!("current_version={current}");
            println!("required_version={required}");
        }
    }

    shared::telemetry::shutdown_tracing();
    Ok(())
}

fn parse_command<I>(mut args: I) -> anyhow::Result<Command>
where
    I: Iterator<Item = String>,
{
    match args.next().as_deref() {
        None | Some("up") => Ok(Command::Up),
        Some("rollback") | Some("down") => Ok(Command::Rollback),
        Some("reset") => Ok(Command::Reset),
        Some("verify") => Ok(Command::Verify),
        Some("version") => Ok(Command::Version),
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            std::process::exit(0);
        }
        Some(other) => {
            print_help();
            bail!("unknown command: {other}");
        }
    }
}

fn print_help() {
    println!("db-migrator - database migration utility");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin db-migrator -- [up|rollback|reset|verify|version]");
    println!();
    println!("COMMANDS:");
    println!("  up       run migrate up and enforce version gate");
    println!("  rollback revert last applied migration (requires down migration)");
    println!("  reset    drop database, create database, then migrate up");
    println!("  verify   only run version gate check");
    println!("  version  print current and required version");
}

async fn rollback_latest(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let current_version = fetch_current_db_version(pool).await?;
    if current_version == 0 {
        tracing::info!("no applied migration found, rollback skipped");
        return Ok(());
    }

    let mut migrator = load_migrator().await?;
    migrator.set_ignore_missing(true);
    let target = previous_target_version(&migrator, current_version).unwrap_or(0);

    migrator
        .undo(pool, target)
        .await
        .context("failed to rollback migration")?;

    let after_version = fetch_current_db_version(pool).await?;
    if after_version >= current_version {
        bail!(
            "rollback did not change version (current={}, after={}); ensure matching .down.sql exists",
            current_version,
            after_version
        );
    }

    tracing::info!(
        current_version,
        target_version = target,
        after_version,
        "rollback completed"
    );
    Ok(())
}

async fn reset_database_and_migrate(
    config: &MigrationCliConfig,
    settings: &shared::db::MigrationConfig,
) -> anyhow::Result<()> {
    let db_url = &config.database.url;
    let exists = <Postgres as MigrateDatabase>::database_exists(db_url)
        .await
        .context("failed to check database existence")?;

    if exists {
        if let Err(err) = <Postgres as MigrateDatabase>::force_drop_database(db_url).await {
            tracing::warn!(error = %err, "force_drop_database failed, fallback to drop_database");
            <Postgres as MigrateDatabase>::drop_database(db_url)
                .await
                .context("failed to drop database")?;
        }
    }

    <Postgres as MigrateDatabase>::create_database(db_url)
        .await
        .context("failed to create database")?;

    let pool = connect_with_retry(db_url, config.database.pool_size, settings)
        .await
        .context("failed to connect database after reset")?;
    let status = run_startup_migration_or_verify(&pool, settings).await?;
    tracing::info!(
        role = ?status.role,
        current_version = status.current_version,
        required_version = status.required_version,
        "reset and migrate completed"
    );
    Ok(())
}

async fn load_migrator() -> anyhow::Result<Migrator> {
    let migration_dir = resolve_migrations_dir()?;
    let migrator = Migrator::new(migration_dir.as_path())
        .await
        .with_context(|| format!("failed to load migrations from {}", migration_dir.display()))?;
    Ok(migrator)
}

fn previous_target_version(migrator: &Migrator, current_version: i64) -> Option<i64> {
    migrator
        .iter()
        .filter(|m| m.migration_type.is_up_migration())
        .map(|m| m.version)
        .filter(|v| *v < current_version)
        .max()
}
