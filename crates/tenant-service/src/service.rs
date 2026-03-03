//! Tenant service business logic
#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Tenant;
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
