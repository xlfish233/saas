//! Tenant quota definitions and management
//!
//! Defines quota limits for different subscription tiers and provides
//! utilities for quota checking.

use serde::{Deserialize, Serialize};

/// Resource types that can be quota-limited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaResource {
    /// Number of users in the tenant
    Users,
    /// Storage usage in bytes
    Storage,
    /// API calls per minute
    ApiCalls,
    /// Number of files stored
    StorageFiles,
}

impl std::fmt::Display for QuotaResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QuotaResource::Users => write!(f, "users"),
            QuotaResource::Storage => write!(f, "storage"),
            QuotaResource::ApiCalls => write!(f, "api_calls"),
            QuotaResource::StorageFiles => write!(f, "storage_files"),
        }
    }
}

impl std::str::FromStr for QuotaResource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "users" => Ok(Self::Users),
            "storage" => Ok(Self::Storage),
            "api_calls" | "apicalls" => Ok(Self::ApiCalls),
            "storage_files" | "storagefiles" => Ok(Self::StorageFiles),
            _ => Err(format!("Invalid quota resource: {}", s)),
        }
    }
}

/// Quota limits for a tenant tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuota {
    /// Maximum number of users allowed
    pub max_users: u32,
    /// Maximum storage in GB
    pub max_storage_gb: u32,
    /// Maximum API calls per minute
    pub max_api_calls_per_minute: u32,
    /// Maximum number of files allowed
    pub max_storage_files: u64,
}

impl Default for TenantQuota {
    fn default() -> Self {
        Self::for_tier("starter")
    }
}

impl TenantQuota {
    /// Create a new quota with specified limits
    #[allow(dead_code)]
    pub fn new(
        max_users: u32,
        max_storage_gb: u32,
        max_api_calls_per_minute: u32,
        max_storage_files: u64,
    ) -> Self {
        Self {
            max_users,
            max_storage_gb,
            max_api_calls_per_minute,
            max_storage_files,
        }
    }

    /// Get quota limits for a subscription tier
    pub fn for_tier(tier: &str) -> Self {
        match tier.to_lowercase().as_str() {
            "starter" => Self {
                max_users: 5,
                max_storage_gb: 10,
                max_api_calls_per_minute: 100,
                max_storage_files: 1000,
            },
            "pro" => Self {
                max_users: 50,
                max_storage_gb: 100,
                max_api_calls_per_minute: 500,
                max_storage_files: 10000,
            },
            "enterprise" => Self {
                max_users: u32::MAX,
                max_storage_gb: u32::MAX,
                max_api_calls_per_minute: 2000,
                max_storage_files: u64::MAX,
            },
            _ => Self::default(),
        }
    }

    /// Get quota limits from a plan enum
    pub fn from_plan(plan: shared::tenant::Plan) -> Self {
        Self::for_tier(&plan.to_string())
    }

    /// Get max storage in bytes
    pub fn max_storage_bytes(&self) -> u64 {
        (self.max_storage_gb as u64) * 1_000_000_000
    }
}

/// Current usage status for a specific resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    /// Resource type
    pub resource: QuotaResource,
    /// Current usage
    pub used: u64,
    /// Maximum allowed
    pub limit: u64,
    /// Whether the quota has been exceeded
    pub exceeded: bool,
    /// Percentage of quota used (0-100)
    pub percentage: f64,
}

impl QuotaStatus {
    /// Create a new quota status
    pub fn new(resource: QuotaResource, used: u64, limit: u64) -> Self {
        let exceeded = used >= limit;
        let percentage = if limit > 0 {
            ((used as f64 / limit as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        Self {
            resource,
            used,
            limit,
            exceeded,
            percentage,
        }
    }

    /// Check if usage is approaching limit (>=80%)
    #[allow(dead_code)]
    pub fn is_warning(&self) -> bool {
        self.percentage >= 80.0 && !self.exceeded
    }
}

/// Complete quota status for a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuotaStatus {
    /// Tenant ID
    pub tenant_id: uuid::Uuid,
    /// Tenant plan/tier
    pub plan: String,
    /// Quota limits
    pub quota: TenantQuota,
    /// User quota status
    pub users: QuotaStatus,
    /// Storage quota status
    pub storage: QuotaStatus,
    /// API calls quota status
    pub api_calls: QuotaStatus,
    /// Storage files quota status
    pub storage_files: QuotaStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_quota_for_tiers() {
        let starter = TenantQuota::for_tier("starter");
        assert_eq!(starter.max_users, 5);
        assert_eq!(starter.max_storage_gb, 10);
        assert_eq!(starter.max_api_calls_per_minute, 100);
        assert_eq!(starter.max_storage_files, 1000);

        let pro = TenantQuota::for_tier("pro");
        assert_eq!(pro.max_users, 50);
        assert_eq!(pro.max_storage_gb, 100);

        let enterprise = TenantQuota::for_tier("enterprise");
        assert_eq!(enterprise.max_users, u32::MAX);
    }

    #[test]
    fn test_quota_status() {
        let status = QuotaStatus::new(QuotaResource::Users, 3, 5);
        assert_eq!(status.used, 3);
        assert_eq!(status.limit, 5);
        assert!(!status.exceeded);
        assert_eq!(status.percentage, 60.0);
    }

    #[test]
    fn test_quota_exceeded() {
        let status = QuotaStatus::new(QuotaResource::Users, 5, 5);
        assert!(status.exceeded);
        assert_eq!(status.percentage, 100.0);
    }

    #[test]
    fn test_quota_warning_level() {
        let status = QuotaStatus::new(QuotaResource::Users, 4, 5);
        assert!(status.is_warning());
        assert!(!status.exceeded);
    }

    #[test]
    fn test_storage_bytes_conversion() {
        let quota = TenantQuota::for_tier("starter");
        assert_eq!(quota.max_storage_bytes(), 10_000_000_000); // 10GB in bytes
    }

    #[test]
    fn test_resource_from_str() {
        assert!(QuotaResource::from_str("users").is_ok());
        assert!(QuotaResource::from_str("storage").is_ok());
        assert!(QuotaResource::from_str("api_calls").is_ok());
        assert!(QuotaResource::from_str("invalid").is_err());
    }

    #[test]
    fn test_default_quota() {
        let quota = TenantQuota::default();
        assert_eq!(quota.max_users, 5); // Should be starter tier
    }
}
