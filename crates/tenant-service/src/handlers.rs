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
