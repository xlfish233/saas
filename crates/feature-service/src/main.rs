//! Feature Flag Service - Feature Toggle Management
//!
//! Handles feature flag lifecycle, tenant feature assignments, and runtime checks.

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

use service::FeatureService;
use shared::config::AppConfig;

#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    config: Arc<AppConfig>,
    db: sqlx::PgPool,
    feature_service: Arc<FeatureService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;
    shared::telemetry::init_tracing("feature-service");

    tracing::info!("Starting feature service...");

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

    let feature_service = Arc::new(FeatureService::new(db.clone()));

    let state = AppState {
        config: Arc::new(config),
        db,
        feature_service,
    };

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);

    let app = Router::new()
        // Health endpoints
        .route("/health", get(health))
        .route("/ready", get(ready))
        // Feature flag management
        .route("/features", get(handlers::list_features))
        .route("/features", post(handlers::create_feature))
        .route("/features/:id", get(handlers::get_feature))
        .route("/features/:id", put(handlers::update_feature))
        .route("/features/:id", delete(handlers::delete_feature))
        // Tenant feature management
        .route("/tenants/:id/features", get(handlers::list_tenant_features))
        .route(
            "/tenants/:id/features/:featureId",
            post(handlers::enable_tenant_feature),
        )
        .route(
            "/tenants/:id/features/:featureId",
            delete(handlers::disable_tenant_feature),
        )
        // Runtime feature check
        .route("/check-feature", get(handlers::check_feature))
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
