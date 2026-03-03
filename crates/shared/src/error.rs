//! Error types for the ERP SaaS platform

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Token revoked")]
    TokenRevoked,

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("Tenant not found: {0}")]
    TenantNotFound(uuid::Uuid),

    #[error("Tenant inactive: {0}")]
    TenantInactive(uuid::Uuid),

    #[error("User not found: {0}")]
    UserNotFound(uuid::Uuid),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("NATS error: {0}")]
    Nats(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal server error")]
    Internal,

    #[error("{0}")]
    Other(String),
}

impl From<std::env::VarError> for Error {
    fn from(e: std::env::VarError) -> Self {
        Error::Config(e.to_string())
    }
}

impl From<config::ConfigError> for Error {
    fn from(e: config::ConfigError) -> Self {
        Error::Config(e.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match e.kind() {
            ErrorKind::ExpiredSignature => Error::TokenExpired,
            _ => Error::Auth(e.to_string()),
        }
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        use axum::Json;
        use serde_json::json;

        let (status, message) = match &self {
            Error::InvalidCredentials | Error::TokenExpired | Error::TokenRevoked => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            Error::Unauthorized | Error::PermissionDenied(_) => {
                (StatusCode::FORBIDDEN, self.to_string())
            }
            Error::TenantNotFound(_) | Error::UserNotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            Error::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            Error::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": status.canonical_reason().unwrap_or("Error"),
            "message": message
        }));

        (status, body).into_response()
    }
}
