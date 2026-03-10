//! HTTP handlers for tenant endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::models::*;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub isolation_level: String,
    pub plan: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub plan: Option<String>,
    pub is_active: Option<bool>,
}

/// List all tenants (admin only)
pub async fn list_tenants(
    State(state): State<AppState>,
) -> Result<Json<Vec<TenantResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let tenants = state.tenant_service.list_tenants().await.map_err(|e| {
        tracing::error!("Failed to list tenants: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("Internal error")),
        )
    })?;

    Ok(Json(
        tenants.into_iter().map(TenantResponse::from).collect(),
    ))
}

/// Create a new tenant
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, (StatusCode, Json<ErrorResponse>)> {
    let isolation =
        shared::tenant::IsolationLevel::from_str(&req.isolation_level).map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid isolation level")),
            )
        })?;

    let plan = shared::tenant::Plan::from_str(&req.plan).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Invalid plan")),
        )
    })?;

    let tenant = state
        .tenant_service
        .create_tenant(&req.name, &req.slug, isolation, plan)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create tenant: {}", e);
            match e.downcast_ref::<sqlx::Error>() {
                Some(sqlx::Error::Database(db_err))
                    if db_err.constraint() == Some("tenants_slug_key") =>
                {
                    (
                        StatusCode::CONFLICT,
                        Json(ErrorResponse::new("Tenant slug already exists")),
                    )
                }
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                ),
            }
        })?;

    Ok(Json(TenantResponse::from(tenant)))
}

/// Get tenant by ID
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TenantResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tenant = state.tenant_service.get_tenant(id).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("Tenant not found")),
        )
    })?;

    Ok(Json(TenantResponse::from(tenant)))
}

/// Update tenant
pub async fn update_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTenantRequest>,
) -> Result<Json<TenantResponse>, (StatusCode, Json<ErrorResponse>)> {
    let plan = req
        .plan
        .as_ref()
        .map(|p| shared::tenant::Plan::from_str(p))
        .transpose()
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid plan")),
            )
        })?;

    let tenant = state
        .tenant_service
        .update_tenant(id, req.name.as_deref(), plan, req.is_active)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Tenant not found")),
            )
        })?;

    Ok(Json(TenantResponse::from(tenant)))
}

/// Delete tenant (soft delete)
pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state.tenant_service.delete_tenant(id).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("Tenant not found")),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create schema for Bridge isolation
pub async fn create_schema(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SchemaResponse>, (StatusCode, Json<ErrorResponse>)> {
    let schema_name = state
        .tenant_service
        .create_tenant_schema(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create schema: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to create schema")),
            )
        })?;

    Ok(Json(SchemaResponse { schema_name }))
}

#[derive(Debug, Serialize)]
pub struct SchemaResponse {
    pub schema_name: String,
}

// ========================================
// Quota Management Handlers
// ========================================

use crate::models::{QuotaResponse, UsageResponse};
use crate::quota::{QuotaResource, TenantQuotaStatus};

/// Get tenant quota configuration
pub async fn get_tenant_quota(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<QuotaResponse>, (StatusCode, Json<ErrorResponse>)> {
    let quota = state.tenant_service.get_quota(id).await.map_err(|e| {
        tracing::error!("Failed to get quota: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("Failed to get quota")),
        )
    })?;

    Ok(Json(QuotaResponse {
        max_users: quota.max_users,
        max_storage_gb: quota.max_storage_gb,
        max_api_calls_per_minute: quota.max_api_calls_per_minute,
        max_storage_files: quota.max_storage_files,
    }))
}

/// Get tenant usage
pub async fn get_tenant_usage(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UsageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let usage = state.tenant_service.get_usage(id).await.map_err(|e| {
        tracing::error!("Failed to get usage: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("Failed to get usage")),
        )
    })?;

    Ok(Json(usage))
}

/// Get complete quota status for a tenant
pub async fn get_tenant_quota_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TenantQuotaStatus>, (StatusCode, Json<ErrorResponse>)> {
    let status = state
        .tenant_service
        .get_quota_status(id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get quota status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to get quota status")),
            )
        })?;

    Ok(Json(status))
}

/// Check if a specific resource quota is exceeded
#[derive(Debug, Deserialize)]
pub struct CheckQuotaPath {
    pub tenant_id: Uuid,
    pub resource: String,
}

pub async fn check_quota(
    State(state): State<AppState>,
    Path(path): Path<CheckQuotaPath>,
) -> Result<Json<crate::quota::QuotaStatus>, (StatusCode, Json<ErrorResponse>)> {
    let resource = QuotaResource::from_str(&path.resource).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Invalid resource type")),
        )
    })?;

    let status = state
        .tenant_service
        .check_quota(path.tenant_id, resource)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check quota: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Failed to check quota")),
            )
        })?;

    // Return 429 if quota exceeded
    if status.exceeded {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse::new(&format!(
                "Quota exceeded for resource: {}",
                resource
            ))),
        ));
    }

    Ok(Json(status))
}
