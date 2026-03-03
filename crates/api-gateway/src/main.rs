//! API Gateway - Main Entry Point

use axum::{
    routing::{get, post},
    Router,
};
use http::{header, HeaderName, Method};
use std::net::SocketAddr;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};

mod config;
mod routes;
mod telemetry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize tracing
    telemetry::init_tracing();

    // Load config
    let config = config::Config::from_env()?;
    tracing::info!(
        "Starting API Gateway on {}:{}",
        config.host(),
        config.port()
    );

    // Configure CORS from environment
    let cors = build_cors_layer(&config);

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh))
        .route("/api/v1/tenants", get(routes::tenants::list))
        .route("/api/v1/tenants/{id}", get(routes::tenants::get))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.host(), config.port()).parse()?;

    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Build CORS layer from configuration
/// In production, CORS_ORIGINS should be set to allowed domains
fn build_cors_layer(config: &config::Config) -> CorsLayer {
    let allowed_origins = match config.cors_origins() {
        Some(origins) if !origins.is_empty() => {
            // Parse comma-separated list of origins
            let origins: Vec<http::HeaderValue> = origins
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            AllowOrigin::list(origins)
        }
        _ => {
            // Default: only allow localhost for development
            tracing::warn!("CORS_ORIGINS not set, using restrictive defaults");
            AllowOrigin::exact(http::HeaderValue::from_static("http://localhost:3000"))
        }
    };

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-tenant-id"),
        ])
}

async fn health() -> &'static str {
    "OK"
}

async fn ready() -> &'static str {
    "Ready"
}
