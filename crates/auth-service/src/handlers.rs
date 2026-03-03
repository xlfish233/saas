//! HTTP handlers for auth endpoints

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::models::*;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub tenant_slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub tenant_id: uuid::Uuid,
}

/// Login endpoint
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = state.auth_service
        .authenticate(&req.email, &req.password, req.tenant_slug.as_deref())
        .await
        .map_err(|e| {
            tracing::warn!("Login failed for {}: {}", req.email, e);
            (StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("Invalid credentials")))
        })?;

    let (access_token, refresh_token) = state.auth_service
        .generate_tokens(&user)
        .await
        .map_err(|e| {
            tracing::error!("Token generation failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Internal error")))
        })?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        expires_in: state.config.jwt.access_token_expiry_seconds,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            role: user.role,
            tenant_id: user.tenant_id,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Refresh token endpoint
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (user, new_access_token, new_refresh_token) = state.auth_service
        .refresh_tokens(&req.refresh_token)
        .await
        .map_err(|e| {
            tracing::warn!("Token refresh failed: {}", e);
            (StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("Invalid refresh token")))
        })?;

    Ok(Json(LoginResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
        expires_in: state.config.jwt.access_token_expiry_seconds,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            role: user.role,
            tenant_id: user.tenant_id,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

/// Logout endpoint
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if let Some(token) = req.refresh_token {
        state.auth_service
            .revoke_token(&token)
            .await
            .map_err(|e| {
                tracing::error!("Logout failed: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("Logout failed")))
            })?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Get current user info
pub async fn me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("Missing authorization"))))?;

    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("Invalid authorization header"))));
    }

    let token = &auth_header[7..];
    
    let claims = state.auth_service
        .validate_token(token)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("Invalid token"))))?;

    let user = state.auth_service
        .get_user_by_id(claims.sub)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found"))))?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        role: user.role,
        tenant_id: user.tenant_id,
    }))
}
