//! HTTP handlers for feature flag endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::*;
use crate::service::FeatureError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CheckFeatureQuery {
    pub feature_key: String,
    pub tenant_id: Uuid,
}

/// List all feature flags
pub async fn list_features(
    State(state): State<AppState>,
) -> Result<Json<Vec<FeatureFlagResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let features = state.feature_service.list_features().await.map_err(|e| {
        tracing::error!("Failed to list features: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("Internal error")),
        )
    })?;

    Ok(Json(
        features
            .into_iter()
            .map(FeatureFlagResponse::from)
            .collect(),
    ))
}

/// Get feature by ID
pub async fn get_feature(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let feature = state
        .feature_service
        .get_feature(id)
        .await
        .map_err(|e| match e {
            FeatureError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Feature not found")),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Internal error")),
            ),
        })?;

    Ok(Json(FeatureFlagResponse::from(feature)))
}

/// Create a new feature flag
pub async fn create_feature(
    State(state): State<AppState>,
    Json(req): Json<CreateFeatureRequest>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let feature = state
        .feature_service
        .create_feature(
            &req.key,
            &req.name,
            req.description.as_deref(),
            req.enabled.unwrap_or(true),
            req.required_tier.as_deref(),
            req.rollout_percentage.unwrap_or(100),
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to create feature: {}", e);
            match e {
                FeatureError::DuplicateKey(key) => (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse::new(&format!(
                        "Feature key already exists: {}",
                        key
                    ))),
                ),
                FeatureError::InvalidTier(tier) => (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(&format!("Invalid tier: {}", tier))),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                ),
            }
        })?;

    Ok(Json(FeatureFlagResponse::from(feature)))
}

/// Update a feature flag
pub async fn update_feature(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFeatureRequest>,
) -> Result<Json<FeatureFlagResponse>, (StatusCode, Json<ErrorResponse>)> {
    let feature = state
        .feature_service
        .update_feature(
            id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.enabled,
            req.required_tier.as_deref(),
            req.rollout_percentage,
        )
        .await
        .map_err(|e| match e {
            FeatureError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Feature not found")),
            ),
            FeatureError::InvalidTier(tier) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(&format!("Invalid tier: {}", tier))),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Internal error")),
            ),
        })?;

    Ok(Json(FeatureFlagResponse::from(feature)))
}

/// Delete a feature flag
pub async fn delete_feature(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .feature_service
        .delete_feature(id)
        .await
        .map_err(|e| match e {
            FeatureError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("Feature not found")),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Internal error")),
            ),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// List all features for a tenant with their enabled status
pub async fn list_tenant_features(
    State(state): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> Result<Json<Vec<TenantFeatureResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let features = state
        .feature_service
        .list_tenant_features(tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list tenant features: {}", e);
            match e {
                FeatureError::TenantNotFound(_) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("Tenant not found")),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                ),
            }
        })?;

    Ok(Json(
        features
            .into_iter()
            .map(|fws| TenantFeatureResponse {
                feature_id: fws.feature.id,
                feature_key: fws.feature.key,
                feature_name: fws.feature.name,
                enabled: fws.tenant_enabled.unwrap_or(false),
            })
            .collect(),
    ))
}

/// Enable a feature for a tenant
pub async fn enable_tenant_feature(
    State(state): State<AppState>,
    Path((tenant_id, feature_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<EnableTenantFeatureRequest>,
) -> Result<Json<TenantFeatureResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _tenant_feature = state
        .feature_service
        .enable_tenant_feature(tenant_id, feature_id, req.enabled)
        .await
        .map_err(|e| {
            tracing::error!("Failed to enable tenant feature: {}", e);
            match e {
                FeatureError::TenantNotFound(_) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("Tenant not found")),
                ),
                FeatureError::NotFound(_) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("Feature not found")),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                ),
            }
        })?;

    // Get the feature to return its details
    let feature = state
        .feature_service
        .get_feature(feature_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Internal error")),
            )
        })?;

    Ok(Json(TenantFeatureResponse {
        feature_id: feature.id,
        feature_key: feature.key,
        feature_name: feature.name,
        enabled: req.enabled,
    }))
}

/// Disable a feature for a tenant
pub async fn disable_tenant_feature(
    State(state): State<AppState>,
    Path((tenant_id, feature_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .feature_service
        .disable_tenant_feature(tenant_id, feature_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to disable tenant feature: {}", e);
            match e {
                FeatureError::TenantNotFound(_) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("Tenant not found")),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                ),
            }
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Check if a feature is enabled for a tenant (runtime check)
pub async fn check_feature(
    State(state): State<AppState>,
    Query(query): Query<CheckFeatureQuery>,
) -> Result<Json<CheckFeatureResponse>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .feature_service
        .is_feature_enabled(&query.feature_key, query.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to check feature: {}", e);
            match e {
                FeatureError::NotFound(key) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(&format!("Feature not found: {}", key))),
                ),
                FeatureError::TenantNotFound(id) => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new(&format!("Tenant not found: {}", id))),
                ),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                ),
            }
        })?;

    Ok(Json(CheckFeatureResponse {
        enabled: result.enabled,
        reason: result.reason,
    }))
}
