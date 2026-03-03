//! Tenant Service - Multi-tenant Management
//!
//! Handles tenant lifecycle, configuration, and isolation management.

use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod handlers;
mod models;
mod repository;
mod service;

use service::TenantService;
use shared::config::AppConfig;

#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    config: Arc<AppConfig>,
    db: sqlx::PgPool,
    tenant_service: Arc<TenantService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;
    shared::telemetry::init_tracing("tenant-service");

    tracing::info!("Starting tenant service...");

    let mut migration_settings = config.database.migration.clone();
    migration_settings.role = shared::db::MigrationRole::Verifier;

    let db = shared::db::connect_with_retry(
        &config.database.url,
        config.database.pool_size,
        &migration_settings,
    )
    .await?;

    let migration_status =
        shared::db::run_startup_migration_or_verify(&db, &migration_settings).await?;
    tracing::info!(
        role = ?migration_status.role,
        current_version = migration_status.current_version,
        required_version = migration_status.required_version,
        "database migration check completed"
    );

    let tenant_service = Arc::new(TenantService::new(db.clone()));

    let state = AppState {
        config: Arc::new(config),
        db,
        tenant_service,
    };

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/tenants", get(handlers::list_tenants))
        .route("/tenants", post(handlers::create_tenant))
        .route("/tenants/:id", get(handlers::get_tenant))
        .route("/tenants/:id", put(handlers::update_tenant))
        .route("/tenants/:id", delete(handlers::delete_tenant))
        .route("/tenants/:id/schema", post(handlers::create_schema))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    shared::telemetry::shutdown_tracing();
    Ok(())
}

async fn health() -> &'static str {
    "OK"
}

async fn ready(State(_state): State<AppState>) -> Result<&'static str, StatusCode> {
    Ok("OK")
}
