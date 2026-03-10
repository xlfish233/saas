//! Data models for feature flag service

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, FromRow, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// Feature flag database model
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FeatureFlag {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub required_tier: Option<String>,
    pub rollout_percentage: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl<'r> FromRow<'r, PgRow> for FeatureFlag {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            key: row.try_get("key")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            enabled: row.try_get("enabled")?,
            required_tier: row.try_get("required_tier")?,
            rollout_percentage: row.try_get("rollout_percentage")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

/// Feature flag response DTO
#[derive(Debug, Serialize)]
pub struct FeatureFlagResponse {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub required_tier: Option<String>,
    pub rollout_percentage: i32,
}

impl From<FeatureFlag> for FeatureFlagResponse {
    fn from(f: FeatureFlag) -> Self {
        Self {
            id: f.id,
            key: f.key,
            name: f.name,
            description: f.description,
            enabled: f.enabled,
            required_tier: f.required_tier,
            rollout_percentage: f.rollout_percentage,
        }
    }
}

/// Tenant feature assignment database model
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TenantFeature {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub feature_id: Uuid,
    pub enabled: bool,
    pub created_at: OffsetDateTime,
}

impl<'r> FromRow<'r, PgRow> for TenantFeature {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            feature_id: row.try_get("feature_id")?,
            enabled: row.try_get("enabled")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// Tenant feature response DTO
#[derive(Debug, Serialize)]
pub struct TenantFeatureResponse {
    pub feature_id: Uuid,
    pub feature_key: String,
    pub feature_name: String,
    pub enabled: bool,
}

/// Feature check request
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CheckFeatureRequest {
    pub feature_key: String,
    pub tenant_id: Uuid,
}

/// Feature check response
#[derive(Debug, Serialize)]
pub struct CheckFeatureResponse {
    pub enabled: bool,
    pub reason: String,
}

/// Create feature flag request
#[derive(Debug, Deserialize)]
pub struct CreateFeatureRequest {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub required_tier: Option<String>,
    pub rollout_percentage: Option<i32>,
}

/// Update feature flag request
#[derive(Debug, Deserialize)]
pub struct UpdateFeatureRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub required_tier: Option<String>,
    pub rollout_percentage: Option<i32>,
}

/// Enable tenant feature request
#[derive(Debug, Deserialize)]
pub struct EnableTenantFeatureRequest {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
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

/// Feature flag with tenant override status
#[derive(Debug, Clone)]
pub struct FeatureWithTenantStatus {
    pub feature: FeatureFlag,
    pub tenant_enabled: Option<bool>,
}
