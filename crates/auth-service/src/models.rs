//! Data models for auth service

use serde::Serialize;
use sqlx::{postgres::PgRow, FromRow, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// User model
#[derive(Debug, Clone)]
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

impl<'r> FromRow<'r, PgRow> for User {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            email: row.try_get("email")?,
            password_hash: row.try_get("password_hash")?,
            name: row.try_get("name")?,
            role: row.try_get("role")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Tenant model
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub isolation_level: String,
    pub plan: String,
    pub is_active: bool,
}

impl<'r> FromRow<'r, PgRow> for Tenant {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            slug: row.try_get("slug")?,
            isolation_level: row.try_get("isolation_level")?,
            plan: row.try_get("plan")?,
            is_active: row.try_get("is_active")?,
        })
    }
}

/// Refresh token model
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

impl<'r> FromRow<'r, PgRow> for RefreshToken {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            token_hash: row.try_get("token_hash")?,
            expires_at: row.try_get("expires_at")?,
            created_at: row.try_get("created_at")?,
        })
    }
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
