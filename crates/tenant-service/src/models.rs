//! Data models for tenant service

use serde::{Deserialize, Serialize};
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

/// Tenant usage tracking model
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TenantUsage {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_count: i32,
    pub storage_used_bytes: i64,
    pub api_calls_count: i64,
    pub storage_files_count: i64,
    pub period_start: OffsetDateTime,
    pub period_end: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl<'r> FromRow<'r, PgRow> for TenantUsage {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            user_count: row.try_get("user_count")?,
            storage_used_bytes: row.try_get("storage_used_bytes")?,
            api_calls_count: row.try_get("api_calls_count")?,
            storage_files_count: row.try_get("storage_files_count")?,
            period_start: row.try_get("period_start")?,
            period_end: row.try_get("period_end")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Tenant usage response DTO
#[derive(Debug, Serialize)]
pub struct UsageResponse {
    pub user_count: i32,
    pub storage_used_bytes: i64,
    pub storage_used_gb: f64,
    pub api_calls_count: i64,
    pub storage_files_count: i64,
    pub period_start: OffsetDateTime,
    pub period_end: OffsetDateTime,
}

impl From<TenantUsage> for UsageResponse {
    fn from(u: TenantUsage) -> Self {
        Self {
            user_count: u.user_count,
            storage_used_bytes: u.storage_used_bytes,
            storage_used_gb: u.storage_used_bytes as f64 / 1_000_000_000.0,
            api_calls_count: u.api_calls_count,
            storage_files_count: u.storage_files_count,
            period_start: u.period_start,
            period_end: u.period_end,
        }
    }
}

/// Quota configuration response DTO
#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaResponse {
    pub max_users: u32,
    pub max_storage_gb: u32,
    pub max_api_calls_per_minute: u32,
    pub max_storage_files: u64,
}
