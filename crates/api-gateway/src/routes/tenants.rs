//! Tenant Routes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub isolation_level: String,
    pub plan: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantList {
    pub tenants: Vec<Tenant>,
    pub total: u64,
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    responses(
        (status = 200, description = "List of tenants", body = TenantList)
    )
)]
pub async fn list(State(_state): State<AppState>) -> Json<TenantList> {
    // TODO: Implement actual tenant listing
    // Can now access state.jwt_service, state.config, state.http_client
    Json(TenantList {
        tenants: vec![],
        total: 0,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    responses(
        (status = 200, description = "Tenant details", body = Tenant),
        (status = 404, description = "Tenant not found")
    )
)]
pub async fn get(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<Tenant>, StatusCode> {
    // TODO: Implement actual tenant retrieval
    Err(StatusCode::NOT_IMPLEMENTED)
}
