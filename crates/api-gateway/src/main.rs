//! API Gateway - Main Entry Point

use axum::{
    routing::{get, post},
    Router,
};
use http::{header, HeaderName, Method};
use shared::auth::JwtService;
use shared::middleware::RateLimiter;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};

mod auth_proxy;
mod config;
mod routes;
mod telemetry;

/// Application state shared across handlers
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<config::Config>,
    pub(crate) jwt_service: Arc<JwtService>,
    pub(crate) http_client: reqwest::Client,
    #[allow(dead_code)]
    pub(crate) rate_limiter: RateLimiter,
}

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
        config.server.host,
        config.server.port
    );

    // Initialize JWT service for token validation
    let jwt_service = JwtService::from_files(
        &config.jwt.private_key_path,
        &config.jwt.public_key_path,
        config.jwt.issuer.clone(),
        config.jwt.audience.clone(),
        config.jwt.access_token_expiry_seconds as i64,
        config.jwt.refresh_token_expiry_seconds as i64,
    )?;
    tracing::info!("JWT service initialized");

    // Create HTTP client for proxying to auth-service
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    // Create rate limiter
    let rate_limiter = shared::middleware::default_rate_limiter();

    // Build state
    let state = AppState {
        config: Arc::new(config.clone()),
        jwt_service: Arc::new(jwt_service),
        http_client,
        rate_limiter,
    };

    // Configure CORS from environment
    let cors = build_cors_layer(&config);

    // Build router with layered middleware
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        // Auth routes (public, but rate-limited)
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh))
        .route("/api/v1/auth/logout", post(routes::auth::logout))
        // Protected routes
        .route("/api/v1/auth/me", get(routes::auth::me))
        .route("/api/v1/tenants", get(routes::tenants::list))
        .route("/api/v1/tenants/{id}", get(routes::tenants::get))
        .with_state(state)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;

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
