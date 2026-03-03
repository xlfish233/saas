//! Tenant context management

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant isolation levels
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IsolationLevel {
    /// Shared database, shared schema (logical isolation via tenant_id)
    #[default]
    Pool,
    /// Shared database, separate schema per tenant
    Bridge,
    /// Separate database per tenant
    Silo,
}

impl std::fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationLevel::Pool => write!(f, "pool"),
            IsolationLevel::Bridge => write!(f, "bridge"),
            IsolationLevel::Silo => write!(f, "silo"),
        }
    }
}

impl std::str::FromStr for IsolationLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pool" => Ok(Self::Pool),
            "bridge" => Ok(Self::Bridge),
            "silo" => Ok(Self::Silo),
            _ => Err(format!("Invalid isolation level: {}", s)),
        }
    }
}

/// Tenant subscription plans
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    #[default]
    Starter,
    Pro,
    Enterprise,
}

impl std::fmt::Display for Plan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Plan::Starter => write!(f, "starter"),
            Plan::Pro => write!(f, "pro"),
            Plan::Enterprise => write!(f, "enterprise"),
        }
    }
}

/// Tenant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub isolation_level: IsolationLevel,
    pub plan: Plan,
    pub is_active: bool,
    pub schema_name: Option<String>,
    pub database_url: Option<String>,
}

/// Tenant context carried in requests
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant: Tenant,
    pub user_id: Uuid,
    pub role: String,
    pub permissions: Vec<String>,
}

impl TenantContext {
    pub fn new(tenant: Tenant, user_id: Uuid, role: String, permissions: Vec<String>) -> Self {
        Self {
            tenant,
            user_id,
            role,
            permissions,
        }
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| {
            p == permission || 
            (p.ends_with(":*") && permission.starts_with(&p[..p.len() - 1]))
        })
    }

    /// Check if tenant is on a specific plan or higher
    pub fn has_plan(&self, required: Plan) -> bool {
        match (&self.tenant.plan, required) {
            (Plan::Enterprise, _) => true,
            (Plan::Pro, Plan::Enterprise) => false,
            (Plan::Pro, _) => true,
            (Plan::Starter, Plan::Starter) => true,
            (Plan::Starter, _) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_check() {
        let ctx = TenantContext {
            tenant: Tenant {
                id: Uuid::nil(),
                name: "Test".to_string(),
                slug: "test".to_string(),
                isolation_level: IsolationLevel::Pool,
                plan: Plan::Pro,
                is_active: true,
                schema_name: None,
                database_url: None,
            },
            user_id: Uuid::nil(),
            role: "admin".to_string(),
            permissions: vec!["users:read".to_string(), "finance:*".to_string()],
        };

        assert!(ctx.has_permission("users:read"));
        assert!(ctx.has_permission("finance:read"));
        assert!(ctx.has_permission("finance:write"));
        assert!(!ctx.has_permission("users:write"));
        assert!(!ctx.has_permission("admin:all"));
    }

    #[test]
    fn test_plan_hierarchy() {
        let enterprise_tenant = Tenant {
            id: Uuid::nil(),
            name: "Enterprise".to_string(),
            slug: "enterprise".to_string(),
            isolation_level: IsolationLevel::Silo,
            plan: Plan::Enterprise,
            is_active: true,
            schema_name: None,
            database_url: None,
        };

        assert!(enterprise_tenant.plan >= Plan::Pro);
        assert!(enterprise_tenant.plan >= Plan::Starter);
    }
}
