//! Database repository for feature flag data
#![allow(dead_code)]

use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{FeatureFlag, FeatureWithTenantStatus, TenantFeature};

pub struct FeatureRepository {
    pool: PgPool,
}

impl FeatureRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ==================== Feature Flag Operations ====================

    /// List all feature flags
    pub async fn list_all_features(&self) -> Result<Vec<FeatureFlag>, sqlx::Error> {
        sqlx::query_as::<_, FeatureFlag>(
            r#"
            SELECT id, key, name, description, enabled, required_tier, rollout_percentage, created_at, updated_at
            FROM features
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find feature by ID
    pub async fn find_feature_by_id(&self, id: Uuid) -> Result<Option<FeatureFlag>, sqlx::Error> {
        sqlx::query_as::<_, FeatureFlag>(
            r#"
            SELECT id, key, name, description, enabled, required_tier, rollout_percentage, created_at, updated_at
            FROM features
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find feature by key
    pub async fn find_feature_by_key(&self, key: &str) -> Result<Option<FeatureFlag>, sqlx::Error> {
        sqlx::query_as::<_, FeatureFlag>(
            r#"
            SELECT id, key, name, description, enabled, required_tier, rollout_percentage, created_at, updated_at
            FROM features
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
    }

    /// Create a new feature flag
    pub async fn create_feature(
        &self,
        key: &str,
        name: &str,
        description: Option<&str>,
        enabled: bool,
        required_tier: Option<&str>,
        rollout_percentage: i32,
    ) -> Result<FeatureFlag, sqlx::Error> {
        sqlx::query_as::<_, FeatureFlag>(
            r#"
            INSERT INTO features (key, name, description, enabled, required_tier, rollout_percentage)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, key, name, description, enabled, required_tier, rollout_percentage, created_at, updated_at
            "#,
        )
        .bind(key)
        .bind(name)
        .bind(description)
        .bind(enabled)
        .bind(required_tier)
        .bind(rollout_percentage)
        .fetch_one(&self.pool)
        .await
    }

    /// Update a feature flag
    pub async fn update_feature(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        enabled: Option<bool>,
        required_tier: Option<&str>,
        rollout_percentage: Option<i32>,
    ) -> Result<FeatureFlag, sqlx::Error> {
        sqlx::query_as::<_, FeatureFlag>(
            r#"
            UPDATE features
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                enabled = COALESCE($4, enabled),
                required_tier = COALESCE($5, required_tier),
                rollout_percentage = COALESCE($6, rollout_percentage),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, key, name, description, enabled, required_tier, rollout_percentage, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(enabled)
        .bind(required_tier)
        .bind(rollout_percentage)
        .fetch_one(&self.pool)
        .await
    }

    /// Delete a feature flag
    pub async fn delete_feature(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM features
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ==================== Tenant Feature Operations ====================

    /// List all features for a tenant with their enabled status
    pub async fn list_tenant_features(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<FeatureWithTenantStatus>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                f.id, f.key, f.name, f.description, f.enabled, f.required_tier, f.rollout_percentage, f.created_at, f.updated_at,
                tf.enabled as tenant_enabled
            FROM features f
            LEFT JOIN tenant_features tf ON f.id = tf.feature_id AND tf.tenant_id = $1
            ORDER BY f.created_at ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|row: PgRow| {
                let feature = FeatureFlag {
                    id: row.get("id"),
                    key: row.get("key"),
                    name: row.get("name"),
                    description: row.get("description"),
                    enabled: row.get("enabled"),
                    required_tier: row.get("required_tier"),
                    rollout_percentage: row.get("rollout_percentage"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                };
                let tenant_enabled: Option<bool> = row.get("tenant_enabled");
                FeatureWithTenantStatus {
                    feature,
                    tenant_enabled,
                }
            })
            .collect();

        Ok(results)
    }

    /// Get enabled features for a tenant
    pub async fn get_enabled_features_for_tenant(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<FeatureFlag>, sqlx::Error> {
        sqlx::query_as::<_, FeatureFlag>(
            r#"
            SELECT f.id, f.key, f.name, f.description, f.enabled, f.required_tier, f.rollout_percentage, f.created_at, f.updated_at
            FROM features f
            INNER JOIN tenant_features tf ON f.id = tf.feature_id
            WHERE tf.tenant_id = $1 AND tf.enabled = true AND f.enabled = true
            ORDER BY f.created_at ASC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Enable a feature for a tenant (create or update)
    pub async fn enable_tenant_feature(
        &self,
        tenant_id: Uuid,
        feature_id: Uuid,
        enabled: bool,
    ) -> Result<TenantFeature, sqlx::Error> {
        sqlx::query_as::<_, TenantFeature>(
            r#"
            INSERT INTO tenant_features (tenant_id, feature_id, enabled)
            VALUES ($1, $2, $3)
            ON CONFLICT (tenant_id, feature_id)
            DO UPDATE SET enabled = $3
            RETURNING id, tenant_id, feature_id, enabled, created_at
            "#,
        )
        .bind(tenant_id)
        .bind(feature_id)
        .bind(enabled)
        .fetch_one(&self.pool)
        .await
    }

    /// Disable a feature for a tenant (delete)
    pub async fn disable_tenant_feature(
        &self,
        tenant_id: Uuid,
        feature_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM tenant_features
            WHERE tenant_id = $1 AND feature_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(feature_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Check if a feature is enabled for a tenant
    pub async fn is_feature_enabled_for_tenant(
        &self,
        feature_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<bool>, sqlx::Error> {
        let result: Option<(bool,)> = sqlx::query_as(
            r#"
            SELECT enabled FROM tenant_features
            WHERE tenant_id = $1 AND feature_id = $2
            "#,
        )
        .bind(tenant_id)
        .bind(feature_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(b,)| b))
    }

    /// Get tenant's plan
    pub async fn get_tenant_plan(&self, tenant_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT plan FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(p,)| p))
    }
}
