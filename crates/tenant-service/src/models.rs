//! Data models for tenant service

use serde::Serialize;
use sqlx::{postgres::PgRow, FromRow, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// Tenant database model
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub isolation_level: String,
    pub plan: String,
    pub is_active: bool,
    pub schema_name: Option<String>,
    pub database_url: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
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
            schema_name: row.try_get("schema_name")?,
            database_url: row.try_get("database_url")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Tenant response DTO
#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub isolation_level: String,
    pub plan: String,
    pub is_active: bool,
    pub schema_name: Option<String>,
}

impl From<Tenant> for TenantResponse {
    fn from(t: Tenant) -> Self {
        Self {
            id: t.id,
            name: t.name,
            slug: t.slug,
            isolation_level: t.isolation_level,
            plan: t.plan,
            is_active: t.is_active,
            schema_name: t.schema_name,
        }
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
