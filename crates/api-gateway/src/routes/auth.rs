//! Auth Routes - Proxy to auth-service

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth_proxy::{
    AuthProxy, LoginRequest as ProxyLoginRequest, LogoutRequest as ProxyLogoutRequest,
    RefreshRequest as ProxyRefreshRequest, RegisterRequest as ProxyRegisterRequest,
};
use crate::AppState;

// ============ Request DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub tenant_slug: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub tenant_id: Uuid,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

// ============ Response DTOs ============

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub tenant_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl From<crate::auth_proxy::ProxyError> for ErrorResponse {
    fn from(e: crate::auth_proxy::ProxyError) -> Self {
        Self {
            error: "auth_error".to_string(),
            message: e.to_string(),
        }
    }
}

// ============ Handlers ============

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let proxy = AuthProxy::new(
        state.http_client.clone(),
        state.config.auth_service_url().to_string(),
    );

    let response = proxy
        .login(&ProxyLoginRequest {
            email: payload.email,
            password: payload.password,
            tenant_slug: payload.tenant_slug,
        })
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(ErrorResponse::from(e))))?;

    Ok(Json(LoginResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_in: response.expires_in,
        user: UserResponse {
            id: response.user.id,
            email: response.user.email,
            name: response.user.name,
            role: response.user.role,
            tenant_id: response.user.tenant_id,
        },
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "Registration successful", body = LoginResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let proxy = AuthProxy::new(
        state.http_client.clone(),
        state.config.auth_service_url().to_string(),
    );

    let response = proxy
        .register(&ProxyRegisterRequest {
            email: payload.email,
            password: payload.password,
            name: payload.name,
            tenant_id: payload.tenant_id,
            role: payload.role.unwrap_or_else(|| "user".to_string()),
        })
        .await
        .map_err(|e| {
            let status = match &e {
                crate::auth_proxy::ProxyError::Request(_) => StatusCode::BAD_GATEWAY,
                crate::auth_proxy::ProxyError::Auth(_) => StatusCode::BAD_REQUEST,
            };
            (status, Json(ErrorResponse::from(e)))
        })?;

    Ok(Json(LoginResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_in: response.expires_in,
        user: UserResponse {
            id: response.user.id,
            email: response.user.email,
            name: response.user.name,
            role: response.user.role,
            tenant_id: response.user.tenant_id,
        },
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed", body = LoginResponse),
        (status = 401, description = "Invalid refresh token", body = ErrorResponse)
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let proxy = AuthProxy::new(
        state.http_client.clone(),
        state.config.auth_service_url().to_string(),
    );

    let response = proxy
        .refresh(&ProxyRefreshRequest {
            refresh_token: payload.refresh_token,
        })
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(ErrorResponse::from(e))))?;

    Ok(Json(LoginResponse {
        access_token: response.access_token,
        refresh_token: response.refresh_token,
        expires_in: response.expires_in,
        user: UserResponse {
            id: response.user.id,
            email: response.user.email,
            name: response.user.name,
            role: response.user.role,
            tenant_id: response.user.tenant_id,
        },
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 204, description = "Logout successful"),
        (status = 401, description = "Error", body = ErrorResponse)
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let proxy = AuthProxy::new(
        state.http_client.clone(),
        state.config.auth_service_url().to_string(),
    );

    proxy
        .logout(&ProxyLogoutRequest {
            refresh_token: payload.refresh_token,
        })
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::from(e)),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user info", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    )
)]
pub async fn me(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Extract Authorization header
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".to_string(),
                message: "Missing authorization header".to_string(),
            }),
        ))?;

    // Validate JWT at gateway level first
    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".to_string(),
                message: "Invalid authorization header format".to_string(),
            }),
        ));
    }

    let token = &auth_header[7..];
    state.jwt_service.validate_token(token).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "unauthorized".to_string(),
                message: "Invalid or expired token".to_string(),
            }),
        )
    })?;

    // Proxy to auth-service for full user info
    let proxy = AuthProxy::new(
        state.http_client.clone(),
        state.config.auth_service_url().to_string(),
    );

    let user = proxy
        .me(auth_header)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, Json(ErrorResponse::from(e))))?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        role: user.role,
        tenant_id: user.tenant_id,
    }))
}
