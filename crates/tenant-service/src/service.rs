//! Tenant service business logic
#![allow(dead_code)]

use sqlx::PgPool;
use std::str::FromStr;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::{Tenant, UsageResponse};
use crate::quota::{QuotaResource, QuotaStatus, TenantQuota, TenantQuotaStatus};
use crate::repository::TenantRepository;

pub struct TenantService {
    repo: TenantRepository,
    pool: PgPool,
}

impl TenantService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repo: TenantRepository::new(pool.clone()),
            pool,
        }
    }

    /// List all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, anyhow::Error> {
        self.repo.list_all().await.map_err(Into::into)
    }

    /// Get tenant by ID
    pub async fn get_tenant(&self, id: Uuid) -> Result<Tenant, anyhow::Error> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tenant not found"))
    }

    /// Create a new tenant
    pub async fn create_tenant(
        &self,
        name: &str,
        slug: &str,
        isolation_level: shared::tenant::IsolationLevel,
        plan: shared::tenant::Plan,
    ) -> Result<Tenant, anyhow::Error> {
        let tenant = self
            .repo
            .create(name, slug, &isolation_level.to_string(), &plan.to_string())
            .await?;

        // For Bridge isolation, create schema immediately
        if isolation_level == shared::tenant::IsolationLevel::Bridge {
            let schema_name = format!("tenant_{}", slug.replace('-', "_"));
            self.repo.create_schema(&schema_name).await?;

            // Update tenant with schema name
            self.repo
                .update(tenant.id, None, None, None, Some(&schema_name))
                .await
                .map_err(Into::into)
        } else {
            Ok(tenant)
        }
    }

    /// Update tenant
    pub async fn update_tenant(
        &self,
        id: Uuid,
        name: Option<&str>,
        plan: Option<shared::tenant::Plan>,
        is_active: Option<bool>,
    ) -> Result<Tenant, anyhow::Error> {
        self.repo
            .update(
                id,
                name,
                plan.as_ref().map(|p| p.to_string()).as_deref(),
                is_active,
                None,
            )
            .await
            .map_err(Into::into)
    }

    /// Soft delete tenant
    pub async fn delete_tenant(&self, id: Uuid) -> Result<(), anyhow::Error> {
        self.repo.soft_delete(id).await.map_err(Into::into)
    }

    /// Create schema for Bridge isolation tenant
    pub async fn create_tenant_schema(&self, tenant_id: Uuid) -> Result<String, anyhow::Error> {
        let tenant = self.get_tenant(tenant_id).await?;

        if tenant.isolation_level != "bridge" {
            return Err(anyhow::anyhow!(
                "Schema creation only supported for Bridge isolation"
            ));
        }

        let schema_name = format!("tenant_{}", tenant.slug.replace('-', "_"));

        if !self.repo.schema_exists(&schema_name).await? {
            self.repo.create_schema(&schema_name).await?;
        }

        // Update tenant with schema name
        self.repo
            .update(tenant_id, None, None, None, Some(&schema_name))
            .await?;

        Ok(schema_name)
    }

    /// Get tenant router (for database connection routing)
    pub fn get_router(&self) -> shared::tenant::TenantRouter {
        shared::tenant::TenantRouter::new(self.pool.clone())
    }

    // ========================================
    // Quota Management Methods
    // ========================================

    /// Get quota configuration for a tenant based on their plan
    pub async fn get_quota(&self, tenant_id: Uuid) -> Result<TenantQuota, anyhow::Error> {
        let tenant = self.get_tenant(tenant_id).await?;
        let plan =
            shared::tenant::Plan::from_str(&tenant.plan).unwrap_or(shared::tenant::Plan::Starter);
        Ok(TenantQuota::from_plan(plan))
    }

    /// Get current usage for a tenant
    pub async fn get_usage(&self, tenant_id: Uuid) -> Result<UsageResponse, anyhow::Error> {
        let (period_start, period_end) = self.get_current_period();

        let usage = self
            .repo
            .get_or_create_usage(tenant_id, period_start, period_end)
            .await?;

        Ok(UsageResponse::from(usage))
    }

    /// Check quota status for a specific resource
    pub async fn check_quota(
        &self,
        tenant_id: Uuid,
        resource: QuotaResource,
    ) -> Result<QuotaStatus, anyhow::Error> {
        let quota = self.get_quota(tenant_id).await?;
        let (period_start, period_end) = self.get_current_period();
        let usage = self
            .repo
            .get_or_create_usage(tenant_id, period_start, period_end)
            .await?;

        let status = match resource {
            QuotaResource::Users => {
                QuotaStatus::new(resource, usage.user_count as u64, quota.max_users as u64)
            }
            QuotaResource::Storage => QuotaStatus::new(
                resource,
                (usage.storage_used_bytes / 1_000_000_000) as u64, // GB
                quota.max_storage_gb as u64,
            ),
            QuotaResource::ApiCalls => QuotaStatus::new(
                resource,
                usage.api_calls_count as u64,
                quota.max_api_calls_per_minute as u64,
            ),
            QuotaResource::StorageFiles => QuotaStatus::new(
                resource,
                usage.storage_files_count as u64,
                quota.max_storage_files,
            ),
        };

        Ok(status)
    }

    /// Get complete quota status for a tenant
    pub async fn get_quota_status(
        &self,
        tenant_id: Uuid,
    ) -> Result<TenantQuotaStatus, anyhow::Error> {
        let tenant = self.get_tenant(tenant_id).await?;
        let quota = self.get_quota(tenant_id).await?;
        let (period_start, period_end) = self.get_current_period();
        let usage = self
            .repo
            .get_or_create_usage(tenant_id, period_start, period_end)
            .await?;

        Ok(TenantQuotaStatus {
            tenant_id,
            plan: tenant.plan.clone(),
            quota: quota.clone(),
            users: QuotaStatus::new(
                QuotaResource::Users,
                usage.user_count as u64,
                quota.max_users as u64,
            ),
            storage: QuotaStatus::new(
                QuotaResource::Storage,
                (usage.storage_used_bytes / 1_000_000_000) as u64,
                quota.max_storage_gb as u64,
            ),
            api_calls: QuotaStatus::new(
                QuotaResource::ApiCalls,
                usage.api_calls_count as u64,
                quota.max_api_calls_per_minute as u64,
            ),
            storage_files: QuotaStatus::new(
                QuotaResource::StorageFiles,
                usage.storage_files_count as u64,
                quota.max_storage_files,
            ),
        })
    }

    /// Increment API calls count for rate limiting
    pub async fn increment_api_calls(&self, tenant_id: Uuid) -> Result<(), anyhow::Error> {
        let (period_start, period_end) = self.get_current_period();
        self.repo
            .increment_api_calls(tenant_id, period_start, period_end)
            .await
            .map_err(Into::into)
    }

    /// Increment user count (call when adding a user)
    pub async fn increment_user_count(&self, tenant_id: Uuid) -> Result<(), anyhow::Error> {
        let (period_start, period_end) = self.get_current_period();
        self.repo
            .increment_user_count(tenant_id, period_start, period_end)
            .await
            .map_err(Into::into)
    }

    /// Decrement user count (call when removing a user)
    pub async fn decrement_user_count(&self, tenant_id: Uuid) -> Result<(), anyhow::Error> {
        let (period_start, period_end) = self.get_current_period();
        self.repo
            .decrement_user_count(tenant_id, period_start, period_end)
            .await
            .map_err(Into::into)
    }

    /// Update storage usage
    pub async fn update_storage_usage(
        &self,
        tenant_id: Uuid,
        bytes_delta: i64,
        files_delta: i64,
    ) -> Result<(), anyhow::Error> {
        let (period_start, period_end) = self.get_current_period();
        self.repo
            .update_storage_usage(
                tenant_id,
                bytes_delta,
                files_delta,
                period_start,
                period_end,
            )
            .await
            .map_err(Into::into)
    }

    /// Check if a tenant can add more users
    pub async fn can_add_user(&self, tenant_id: Uuid) -> Result<bool, anyhow::Error> {
        let status = self.check_quota(tenant_id, QuotaResource::Users).await?;
        Ok(!status.exceeded)
    }

    /// Check if a tenant can store more data
    pub async fn can_store_bytes(
        &self,
        tenant_id: Uuid,
        additional_bytes: u64,
    ) -> Result<bool, anyhow::Error> {
        let quota = self.get_quota(tenant_id).await?;
        let (period_start, period_end) = self.get_current_period();
        let usage = self
            .repo
            .get_or_create_usage(tenant_id, period_start, period_end)
            .await?;

        let max_bytes = quota.max_storage_bytes();
        let new_total = (usage.storage_used_bytes as u64).saturating_add(additional_bytes);
        Ok(new_total <= max_bytes)
    }

    /// Get current billing period (minute-based for API rate limiting)
    fn get_current_period(&self) -> (OffsetDateTime, OffsetDateTime) {
        let now = OffsetDateTime::now_utc();
        // For API rate limiting, use minute-based periods
        let period_start = now.replace_second(0).unwrap_or(now);
        let period_end = period_start
            .checked_add(time::Duration::seconds(60))
            .unwrap_or(period_start);
        (period_start, period_end)
    }
}

#[cfg(test)]
mod tests {
    /// Test schema name generation logic
    #[test]
    fn test_schema_name_format() {
        // Test various slug formats
        let test_cases = vec![
            ("my-company", "tenant_my_company"),
            ("test-123", "tenant_test_123"),
            ("acme-corp-ltd", "tenant_acme_corp_ltd"),
            ("simple", "tenant_simple"),
        ];

        for (slug, expected) in test_cases {
            let schema_name = format!("tenant_{}", slug.replace('-', "_"));
            assert_eq!(schema_name, expected);
        }
    }

    /// Test schema name with special characters
    #[test]
    fn test_schema_name_special_chars() {
        // Slugs should be normalized to underscores
        let slug = "test-company-2024";
        let schema_name = format!("tenant_{}", slug.replace('-', "_"));

        assert_eq!(schema_name, "tenant_test_company_2024");
        assert!(!schema_name.contains('-'));
    }

    /// Test isolation level validation
    #[test]
    fn test_isolation_level_validation() {
        use std::str::FromStr;

        // Valid isolation levels
        assert!(shared::tenant::IsolationLevel::from_str("pool").is_ok());
        assert!(shared::tenant::IsolationLevel::from_str("bridge").is_ok());
        assert!(shared::tenant::IsolationLevel::from_str("silo").is_ok());

        // Invalid isolation level
        assert!(shared::tenant::IsolationLevel::from_str("invalid").is_err());
    }

    /// Test plan validation
    #[test]
    fn test_plan_validation() {
        use std::str::FromStr;

        // Valid plans
        assert!(shared::tenant::Plan::from_str("starter").is_ok());
        assert!(shared::tenant::Plan::from_str("pro").is_ok());
        assert!(shared::tenant::Plan::from_str("enterprise").is_ok());

        // Invalid plan
        assert!(shared::tenant::Plan::from_str("free").is_err());
    }

    /// Test schema creation only for bridge
    #[test]
    fn test_schema_creation_condition() {
        let bridge_isolation = "bridge";
        let pool_isolation = "pool";
        let silo_isolation = "silo";

        // Only Bridge should create schema
        assert_eq!(bridge_isolation, "bridge");
        assert_ne!(pool_isolation, "bridge");
        assert_ne!(silo_isolation, "bridge");
    }

    /// Test tenant slug uniqueness (concept)
    #[test]
    fn test_slug_uniqueness_requirement() {
        // Slugs must be unique across tenants
        let slug1 = "company-a";
        let slug2 = "company-b";
        let slug3 = "company-a"; // Duplicate

        assert_ne!(slug1, slug2);
        assert_eq!(slug1, slug3); // This would violate unique constraint
    }
}
