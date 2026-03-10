//! API Gateway - Main Entry Point

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use http::{header, HeaderName, Method};
use shared::auth::JwtService;
use shared::db::run_startup_migration_or_verify;
use shared::middleware::{
    audit_with_exempt, create_audit_service, default_rate_limiter, rate_limit_with_exempt,
    AuditLogService,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod auth_proxy;
mod config;
mod routes;
mod telemetry;

// ============ OpenAPI Documentation ============

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
        ready,
        routes::auth::login,
        routes::auth::register,
        routes::auth::refresh,
        routes::auth::logout,
        routes::auth::me,
        routes::tenants::list,
        routes::tenants::get,
    ),
    components(
        schemas(
            routes::auth::LoginRequest,
            routes::auth::RegisterRequest,
            routes::auth::RefreshRequest,
            routes::auth::LogoutRequest,
            routes::auth::LoginResponse,
            routes::auth::UserResponse,
            routes::auth::ErrorResponse,
            routes::tenants::Tenant,
            routes::tenants::TenantList,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "tenants", description = "Tenant management endpoints"),
    )
)]
struct ApiDoc;

/// Paths exempt from rate limiting
const RATE_LIMIT_EXEMPT_PATHS: &[&str] = &["/health", "/ready"];

/// Paths exempt from audit logging
const AUDIT_EXEMPT_PATHS: &[&str] = &["/health", "/ready"];

/// Application state shared across handlers
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<config::Config>,
    pub(crate) jwt_service: Arc<JwtService>,
    pub(crate) http_client: reqwest::Client,
    pub(crate) audit_service: AuditLogService,
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

    // Connect to database
    let pool = PgPool::connect(&config.database.url).await?;
    tracing::info!("Database pool created");

    // Run migrations on startup
    run_startup_migration_or_verify(&pool, &config.database.migration).await?;
    tracing::info!("Database migrations verified");

    // Create audit log service
    let audit_service = create_audit_service(pool.clone());
    tracing::info!("Audit log service initialized");

    // Create HTTP client for proxying to auth-service
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    // Create rate limiter with tiered limits
    let rate_limiter = default_rate_limiter();

    // Build state
    let state = AppState {
        config: Arc::new(config.clone()),
        jwt_service: Arc::new(jwt_service),
        http_client,
        audit_service,
    };

    // Configure CORS from environment
    let cors = build_cors_layer(&config);

    // Rate limit exempt paths
    let rate_limit_exempt_paths: Vec<String> = RATE_LIMIT_EXEMPT_PATHS
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Audit exempt paths
    let audit_exempt_paths: Vec<String> =
        AUDIT_EXEMPT_PATHS.iter().map(|s| s.to_string()).collect();

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
        .with_state(state.clone());

    // Add Swagger UI only in non-production environments
    let is_dev = config.server.environment != "production";
    let app = if is_dev {
        tracing::info!("Swagger UI enabled at /swagger-ui");
        Router::new()
            .merge(app)
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    } else {
        app
    };

    let app = app
        // Apply audit logging middleware (health endpoints are exempt)
        .layer(middleware::from_fn_with_state(
            (state.audit_service.clone(), audit_exempt_paths),
            audit_with_exempt,
        ))
        // Apply rate limiting middleware (health endpoints are exempt)
        .layer(middleware::from_fn_with_state(
            (rate_limiter, rate_limit_exempt_paths),
            rate_limit_with_exempt,
        ))
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

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = str)
    ),
    tag = "health"
)]
async fn health() -> &'static str {
    "OK"
}

#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "Service is ready", body = str)
    ),
    tag = "health"
)]
async fn ready() -> &'static str {
    "Ready"
}
