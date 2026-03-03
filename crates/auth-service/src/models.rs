//! Data models for auth service

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

/// User model
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Tenant model
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub isolation_level: String,
    pub plan: String,
    pub is_active: bool,
}

/// Refresh token model
#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(message: &str) -> Self {
        Self {
            error: "error".to_string(),
            message: message.to_string(),
        }
    }
}
