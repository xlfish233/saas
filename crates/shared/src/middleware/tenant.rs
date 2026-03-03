//! Tenant context middleware

use std::sync::Arc;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::tenant::{Tenant, TenantContext};

/// Tenant context middleware
/// 
/// This middleware extracts tenant information from the request and adds
/// it to the request extensions.
pub async fn tenant_middleware(
    axum::extract::State(tenant_loader): axum::extract::State<Arc<dyn TenantLoader>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get tenant ID from header or token claims
    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            // Fallback to subdomain extraction (if using subdomain routing)
            request
                .headers()
                .get("host")
                .and_then(|h| h.to_str().ok())
                .and_then(|host| extract_tenant_from_subdomain(host))
        });

    let tenant_id = match tenant_id {
        Some(id) => id.parse().map_err(|_| StatusCode::BAD_REQUEST)?,
        None => return Err(StatusCode::BAD_REQUEST),
    };

    // Load tenant from database/cache
    let tenant = tenant_loader
        .load_tenant(tenant_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check if tenant is active
    if !tenant.is_active {
        return Err(StatusCode::FORBIDDEN);
    }

    // Store tenant in request extensions
    request.extensions_mut().insert(tenant);

    Ok(next.run(request).await)
}

/// Extract tenant slug from subdomain
fn extract_tenant_from_subdomain(host: &str) -> Option<&str> {
    // Handle formats like: tenant.app.example.com, tenant.localhost:8080
    let host = host.split(':').next()?;
    let parts: Vec<&str> = host.split('.').collect();
    
    if parts.len() >= 2 {
        Some(parts[0])
    } else {
        None
    }
}

/// Tenant loader trait - implement this to load tenants from your data source
#[async_trait::async_trait]
pub trait TenantLoader: Send + Sync + 'static {
    async fn load_tenant(&self, tenant_id: uuid::Uuid) -> Result<Option<Tenant>, Box<dyn std::error::Error>>;
}

/// Get tenant from request
pub fn get_tenant(request: &Request) -> Option<&Tenant> {
    request.extensions().get::<Tenant>()
}

/// Get tenant context from request
pub fn get_tenant_context(request: &Request) -> Option<&TenantContext> {
    request.extensions().get::<TenantContext>()
}
