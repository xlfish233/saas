//! Quota enforcement middleware
//!
//! Enforces tenant resource quotas:
//! - Users: maximum number of users per tenant
//! - Storage: maximum storage in bytes/GB
//! - API calls: maximum API calls per minute
//! - Storage files: maximum number of files

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

/// Resource types for quota checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaResource {
    Users,
    Storage,
    ApiCalls,
    StorageFiles,
}

/// Quota status for a single resource
#[derive(Debug, Clone, Serialize)]
pub struct QuotaStatus {
    pub resource: String,
    pub used: u64,
    pub limit: u64,
    pub exceeded: bool,
    pub percentage: f64,
}

/// Quota exceeded error
#[derive(Debug)]
pub struct QuotaExceededError {
    pub resource: QuotaResource,
    pub used: u64,
    pub limit: u64,
}

impl IntoResponse for QuotaExceededError {
    fn into_response(self) -> Response {
        #[derive(Debug, Serialize)]
        struct ErrorResponse {
            error: String,
            message: String,
            resource: String,
            used: u64,
            limit: u64,
        }

        let resource_name = match self.resource {
            QuotaResource::Users => "users",
            QuotaResource::Storage => "storage",
            QuotaResource::ApiCalls => "api_calls",
            QuotaResource::StorageFiles => "storage_files",
        };

        let body = ErrorResponse {
            error: "quota_exceeded".to_string(),
            message: format!(
                "Quota exceeded for resource '{}'. Used: {}, Limit: {}",
                resource_name, self.used, self.limit
            ),
            resource: resource_name.to_string(),
            used: self.used,
            limit: self.limit,
        };

        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::CONTENT_TYPE, "application/json")],
            axum::Json(body),
        )
            .into_response()
    }
}

/// Trait for quota checking implementations
#[async_trait::async_trait]
pub trait QuotaChecker: Send + Sync + 'static {
    /// Check if the tenant can perform the action
    /// Returns Ok(()) if allowed, Err(QuotaExceededError) if quota exceeded
    async fn check_quota(
        &self,
        tenant_id: Uuid,
        resource: QuotaResource,
    ) -> Result<QuotaStatus, QuotaExceededError>;
}

/// Quota middleware configuration
#[derive(Debug, Clone)]
pub struct QuotaMiddlewareConfig {
    /// Resources to check for each request
    pub check_resources: Vec<QuotaResource>,
    /// Paths exempt from quota checking
    pub exempt_paths: Vec<String>,
}

impl Default for QuotaMiddlewareConfig {
    fn default() -> Self {
        Self {
            check_resources: vec![QuotaResource::ApiCalls],
            exempt_paths: vec![
                "/health".to_string(),
                "/ready".to_string(),
                "/metrics".to_string(),
            ],
        }
    }
}

/// Create quota enforcement middleware
pub fn quota_middleware<C>(
    checker: Arc<C>,
    config: QuotaMiddlewareConfig,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, QuotaExceededError>> + Send>,
> + Clone
       + Send
       + Sync
       + 'static
where
    C: QuotaChecker,
{
    move |request: Request, next: Next| {
        let checker = checker.clone();
        let config = config.clone();

        Box::pin(async move {
            let path = request.uri().path();

            // Check if path is exempt from quota checking
            if config
                .exempt_paths
                .iter()
                .any(|exempt| path == exempt || path.starts_with(exempt))
            {
                return Ok(next.run(request).await);
            }

            // Get tenant ID from request headers/extensions
            let tenant_id = request
                .headers()
                .get("x-tenant-id")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<Uuid>().ok());

            let tenant_id = match tenant_id {
                Some(id) => id,
                None => {
                    // No tenant ID, allow request (will be handled by auth middleware)
                    return Ok(next.run(request).await);
                }
            };

            // Check each configured resource
            for resource in &config.check_resources {
                match checker.check_quota(tenant_id, *resource).await {
                    Ok(status) => {
                        if status.exceeded {
                            return Err(QuotaExceededError {
                                resource: *resource,
                                used: status.used,
                                limit: status.limit,
                            });
                        }
                    }
                    Err(e) => return Err(e),
                }
            }

            Ok(next.run(request).await)
        })
    }
}

/// Simple in-memory quota checker for testing
pub struct InMemoryQuotaChecker {
    quotas: Arc<std::sync::RwLock<std::collections::HashMap<Uuid, (u64, u64)>>>,
}

impl Default for InMemoryQuotaChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryQuotaChecker {
    pub fn new() -> Self {
        Self {
            quotas: Arc::new(std::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn set_quota(&self, tenant_id: Uuid, used: u64, limit: u64) {
        let mut quotas = self.quotas.write().unwrap();
        quotas.insert(tenant_id, (used, limit));
    }
}

#[async_trait::async_trait]
impl QuotaChecker for InMemoryQuotaChecker {
    async fn check_quota(
        &self,
        tenant_id: Uuid,
        _resource: QuotaResource,
    ) -> Result<QuotaStatus, QuotaExceededError> {
        let quotas = self.quotas.read().unwrap();
        let (used, limit) = quotas.get(&tenant_id).copied().unwrap_or((0, u64::MAX));

        let exceeded = used >= limit;
        let percentage = if limit > 0 {
            ((used as f64 / limit as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        let status = QuotaStatus {
            resource: "all".to_string(),
            used,
            limit,
            exceeded,
            percentage,
        };

        if exceeded {
            Err(QuotaExceededError {
                resource: QuotaResource::ApiCalls,
                used,
                limit,
            })
        } else {
            Ok(status)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_exceeded_error_response() {
        let error = QuotaExceededError {
            resource: QuotaResource::Users,
            used: 10,
            limit: 5,
        };

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_quota_middleware_config_default() {
        let config = QuotaMiddlewareConfig::default();

        assert!(config.check_resources.contains(&QuotaResource::ApiCalls));
        assert!(config.exempt_paths.contains(&"/health".to_string()));
    }
}
