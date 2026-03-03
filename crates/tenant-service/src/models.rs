//! Data models for tenant service

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

/// Tenant database model
#[derive(Debug, Clone, sqlx::FromRow)]
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
