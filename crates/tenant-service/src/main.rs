//! Tenant Service - Multi-tenant Management
//!
//! Handles tenant lifecycle, configuration, and isolation management.

use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::postgres::PgPoolOptions;
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

    let db = PgPoolOptions::new()
        .max_connections(config.database.pool_size)
        .connect(&config.database.url)
        .await?;

    sqlx::migrate!("../../migrations").run(&db).await?;

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
