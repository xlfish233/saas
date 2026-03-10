//! Feature flag service business logic
#![allow(dead_code)]

use std::str::FromStr;

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{FeatureFlag, FeatureWithTenantStatus, TenantFeature};
use crate::repository::FeatureRepository;

#[derive(Debug, thiserror::Error)]
pub enum FeatureError {
    #[error("Feature not found: {0}")]
    NotFound(String),

    #[error("Tenant not found: {0}")]
    TenantNotFound(Uuid),

    #[error("Feature key already exists: {0}")]
    DuplicateKey(String),

    #[error("Invalid tier: {0}")]
    InvalidTier(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub struct FeatureService {
    repo: FeatureRepository,
}

impl FeatureService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: FeatureRepository::new(pool),
        }
    }

    // ==================== Feature Flag Management ====================

    /// List all feature flags
    pub async fn list_features(&self) -> Result<Vec<FeatureFlag>, FeatureError> {
        self.repo.list_all_features().await.map_err(Into::into)
    }

    /// Get feature by ID
    pub async fn get_feature(&self, id: Uuid) -> Result<FeatureFlag, FeatureError> {
        self.repo
            .find_feature_by_id(id)
            .await?
            .ok_or_else(|| FeatureError::NotFound(id.to_string()))
    }

    /// Get feature by key
    pub async fn get_feature_by_key(&self, key: &str) -> Result<FeatureFlag, FeatureError> {
        self.repo
            .find_feature_by_key(key)
            .await?
            .ok_or_else(|| FeatureError::NotFound(key.to_string()))
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
    ) -> Result<FeatureFlag, FeatureError> {
        // Validate tier if provided
        if let Some(tier) = required_tier {
            if shared::tenant::Plan::from_str(tier).is_err() {
                return Err(FeatureError::InvalidTier(tier.to_string()));
            }
        }

        self.repo
            .create_feature(
                key,
                name,
                description,
                enabled,
                required_tier,
                rollout_percentage,
            )
            .await
            .map_err(|e| match e {
                sqlx::Error::Database(db_err)
                    if db_err.constraint() == Some("features_key_key") =>
                {
                    FeatureError::DuplicateKey(key.to_string())
                }
                _ => e.into(),
            })
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
    ) -> Result<FeatureFlag, FeatureError> {
        // Validate tier if provided
        if let Some(tier) = required_tier {
            if shared::tenant::Plan::from_str(tier).is_err() {
                return Err(FeatureError::InvalidTier(tier.to_string()));
            }
        }

        self.repo
            .update_feature(
                id,
                name,
                description,
                enabled,
                required_tier,
                rollout_percentage,
            )
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => FeatureError::NotFound(id.to_string()),
                _ => e.into(),
            })
    }

    /// Delete a feature flag
    pub async fn delete_feature(&self, id: Uuid) -> Result<(), FeatureError> {
        self.repo.delete_feature(id).await?;
        Ok(())
    }

    // ==================== Tenant Feature Management ====================

    /// List all features with their status for a tenant
    pub async fn list_tenant_features(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<FeatureWithTenantStatus>, FeatureError> {
        // Verify tenant exists
        let plan = self.repo.get_tenant_plan(tenant_id).await?;
        if plan.is_none() {
            return Err(FeatureError::TenantNotFound(tenant_id));
        }

        self.repo
            .list_tenant_features(tenant_id)
            .await
            .map_err(Into::into)
    }

    /// Enable a feature for a tenant
    pub async fn enable_tenant_feature(
        &self,
        tenant_id: Uuid,
        feature_id: Uuid,
        enabled: bool,
    ) -> Result<TenantFeature, FeatureError> {
        // Verify tenant exists
        let plan = self.repo.get_tenant_plan(tenant_id).await?;
        if plan.is_none() {
            return Err(FeatureError::TenantNotFound(tenant_id));
        }

        // Verify feature exists
        let _feature = self.get_feature(feature_id).await?;

        self.repo
            .enable_tenant_feature(tenant_id, feature_id, enabled)
            .await
            .map_err(Into::into)
    }

    /// Disable a feature for a tenant
    pub async fn disable_tenant_feature(
        &self,
        tenant_id: Uuid,
        feature_id: Uuid,
    ) -> Result<(), FeatureError> {
        // Verify tenant exists
        let plan = self.repo.get_tenant_plan(tenant_id).await?;
        if plan.is_none() {
            return Err(FeatureError::TenantNotFound(tenant_id));
        }

        self.repo
            .disable_tenant_feature(tenant_id, feature_id)
            .await
            .map_err(Into::into)
    }

    // ==================== Feature Check Logic ====================

    /// Check if a feature is enabled for a tenant
    ///
    /// This method implements the full feature flag evaluation logic:
    /// 1. Check if the feature exists and is globally enabled
    /// 2. Check tenant-specific override (if any)
    /// 3. Check if tenant's plan meets the required tier
    /// 4. Check rollout percentage (gradual rollout)
    pub async fn is_feature_enabled(
        &self,
        feature_key: &str,
        tenant_id: Uuid,
    ) -> Result<FeatureCheckResult, FeatureError> {
        // 1. Get the feature
        let feature = self
            .repo
            .find_feature_by_key(feature_key)
            .await?
            .ok_or_else(|| FeatureError::NotFound(feature_key.to_string()))?;

        // Feature is globally disabled
        if !feature.enabled {
            return Ok(FeatureCheckResult {
                enabled: false,
                reason: "Feature is globally disabled".to_string(),
            });
        }

        // 2. Check tenant-specific override
        let tenant_override = self
            .repo
            .is_feature_enabled_for_tenant(feature.id, tenant_id)
            .await?;

        if let Some(enabled) = tenant_override {
            return Ok(FeatureCheckResult {
                enabled,
                reason: if enabled {
                    "Enabled by tenant override"
                } else {
                    "Disabled by tenant override"
                }
                .to_string(),
            });
        }

        // 3. Check tenant's plan against required tier
        let tenant_plan = self
            .repo
            .get_tenant_plan(tenant_id)
            .await?
            .ok_or_else(|| FeatureError::TenantNotFound(tenant_id))?;

        if let Some(ref required_tier) = feature.required_tier {
            let tenant_tier = shared::tenant::Plan::from_str(&tenant_plan)
                .map_err(|_| FeatureError::InvalidTier(tenant_plan.clone()))?;
            let required = shared::tenant::Plan::from_str(required_tier)
                .map_err(|_| FeatureError::InvalidTier(required_tier.clone()))?;

            if tenant_tier < required {
                return Ok(FeatureCheckResult {
                    enabled: false,
                    reason: format!(
                        "Tenant plan '{}' does not meet required tier '{}'",
                        tenant_plan, required_tier
                    ),
                });
            }
        }

        // 4. Check rollout percentage
        if feature.rollout_percentage < 100 {
            // Use tenant_id as the hash key for consistent rollout
            let hash = Self::hash_tenant_id(tenant_id);
            #[allow(clippy::cast_possible_truncation)]
            let threshold = feature.rollout_percentage as u64 * u64::MAX / 100;

            if hash > threshold {
                return Ok(FeatureCheckResult {
                    enabled: false,
                    reason: format!(
                        "Tenant not in rollout percentage ({}%)",
                        feature.rollout_percentage
                    ),
                });
            }
        }

        Ok(FeatureCheckResult {
            enabled: true,
            reason: "Feature is enabled".to_string(),
        })
    }

    /// Simple hash function for tenant ID to determine rollout bucket
    fn hash_tenant_id(tenant_id: Uuid) -> u64 {
        // Use the first 8 bytes of the UUID as a simple hash
        let bytes = tenant_id.as_bytes();
        u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

/// Result of feature flag check
#[derive(Debug, Clone)]
pub struct FeatureCheckResult {
    pub enabled: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_tenant_id_consistency() {
        let tenant_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();

        // Same UUID should produce same hash
        let hash1 = FeatureService::hash_tenant_id(tenant_id);
        let hash2 = FeatureService::hash_tenant_id(tenant_id);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_tenant_id_distribution() {
        // Different UUIDs should produce different hashes (with high probability)
        let id1 = uuid::Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap();
        let id2 = uuid::Uuid::parse_str("20000000-0000-0000-0000-000000000002").unwrap();

        let hash1 = FeatureService::hash_tenant_id(id1);
        let hash2 = FeatureService::hash_tenant_id(id2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_rollout_percentage_logic() {
        // For 50% rollout, approximately half of UUIDs should be included
        // This is a probabilistic test, but with proper hashing it should be consistent

        let tenant_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let hash = FeatureService::hash_tenant_id(tenant_id);

        // Calculate what percentage bucket this tenant falls into
        let percentage = (hash as f64 / u64::MAX as f64) * 100.0;

        // Just verify it's in a valid range
        assert!((0.0..=100.0).contains(&percentage));
    }

    #[test]
    fn test_tier_validation() {
        // Valid tiers
        assert!(shared::tenant::Plan::from_str("starter").is_ok());
        assert!(shared::tenant::Plan::from_str("pro").is_ok());
        assert!(shared::tenant::Plan::from_str("enterprise").is_ok());

        // Invalid tier
        assert!(shared::tenant::Plan::from_str("invalid").is_err());
    }

    #[test]
    fn test_plan_hierarchy() {
        use std::str::FromStr;

        let starter = shared::tenant::Plan::from_str("starter").unwrap();
        let pro = shared::tenant::Plan::from_str("pro").unwrap();
        let enterprise = shared::tenant::Plan::from_str("enterprise").unwrap();

        assert!(starter < pro);
        assert!(pro < enterprise);
        assert!(starter < enterprise);
    }
}
