//! Auth Service - Authentication and Authorization
//!
//! Handles user authentication, token management, and session control.

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod handlers;
mod models;
mod repository;
mod service;

use service::AuthService;
use shared::config::AppConfig;

#[derive(Clone)]
#[allow(dead_code)]
struct AppState {
    config: Arc<AppConfig>,
    db: sqlx::PgPool,
    redis: redis::Client,
    auth_service: Arc<AuthService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let config = AppConfig::load()?;
    shared::telemetry::init_tracing("auth-service");

    tracing::info!("Starting auth service...");

    // Connect to database
    let db = PgPoolOptions::new()
        .max_connections(config.database.pool_size)
        .connect(&config.database.url)
        .await?;

    // Run migrations
    sqlx::migrate!("../../migrations").run(&db).await?;

    // Connect to Redis
    let redis = redis::Client::open(config.redis.url.as_str())?;

    // Initialize auth service
    let auth_service = Arc::new(AuthService::new(
        shared::auth::JwtService::from_files(
            &config.jwt.private_key_path,
            &config.jwt.public_key_path,
            config.jwt.issuer.clone(),
            config.jwt.audience.clone(),
            config.jwt.access_token_expiry_seconds,
            config.jwt.refresh_token_expiry_seconds,
        )?,
        shared::auth::PasswordHasher::new(),
        redis.clone(),
        db.clone(),
    ));

    let state = AppState {
        config: Arc::new(config),
        db,
        redis,
        auth_service,
    };

    // Get address before moving state
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/auth/login", post(handlers::login))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/refresh", post(handlers::refresh))
        .route("/auth/me", get(handlers::me))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
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
