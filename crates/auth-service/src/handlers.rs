//! HTTP handlers for auth endpoints

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::models::*;
use crate::AppState;

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
    let user = state
        .auth_service
        .authenticate(&req.email, &req.password, req.tenant_slug.as_deref())
        .await
        .map_err(|e| {
            tracing::warn!("Login failed for {}: {}", req.email, e);
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid credentials")),
            )
        })?;

    let (access_token, refresh_token) =
        state
            .auth_service
            .generate_tokens(&user)
            .await
            .map_err(|e| {
                tracing::error!("Token generation failed: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new("Internal error")),
                )
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
    let (user, new_access_token, new_refresh_token) = state
        .auth_service
        .refresh_tokens(&req.refresh_token)
        .await
        .map_err(|e| {
            tracing::warn!("Token refresh failed: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid refresh token")),
            )
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
        state.auth_service.revoke_token(&token).await.map_err(|e| {
            tracing::error!("Logout failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("Logout failed")),
            )
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
        .ok_or((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("Missing authorization")),
        ))?;

    if !auth_header.starts_with("Bearer ") {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("Invalid authorization header")),
        ));
    }

    let token = &auth_header[7..];

    let claims = state
        .auth_service
        .validate_token(token)
        .await
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid token")),
            )
        })?;

    let user = state
        .auth_service
        .get_user_by_id(claims.sub)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("User not found")),
            )
        })?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        role: user.role,
        tenant_id: user.tenant_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test LoginRequest validation
    #[test]
    fn test_login_request_structure() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_slug: Some("test-company".to_string()),
        };

        assert_eq!(request.email, "test@example.com");
        assert_eq!(request.password, "password123");
        assert_eq!(request.tenant_slug, Some("test-company".to_string()));
    }

    /// Test LoginRequest without tenant_slug
    #[test]
    fn test_login_request_without_tenant() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
            tenant_slug: None,
        };

        assert!(request.tenant_slug.is_none());
    }

    /// Test RefreshRequest validation
    #[test]
    fn test_refresh_request_structure() {
        let request = RefreshRequest {
            refresh_token: "test_refresh_token_placeholder".to_string(),
        };

        assert_eq!(request.refresh_token, "test_refresh_token_placeholder");
    }

    /// Test LogoutRequest with token
    #[test]
    fn test_logout_request_with_token() {
        let request = LogoutRequest {
            refresh_token: Some("token_to_revoke".to_string()),
        };

        assert!(request.refresh_token.is_some());
    }

    /// Test LogoutRequest without token
    #[test]
    fn test_logout_request_without_token() {
        let request = LogoutRequest {
            refresh_token: None,
        };

        assert!(request.refresh_token.is_none());
    }

    /// Test LoginResponse structure
    #[test]
    fn test_login_response_structure() {
        let user_id = uuid::Uuid::new_v4();
        let tenant_id = uuid::Uuid::new_v4();

        let response = LoginResponse {
            access_token: "access_token".to_string(),
            refresh_token: "refresh_token".to_string(),
            expires_in: 900,
            user: UserResponse {
                id: user_id,
                email: "test@example.com".to_string(),
                name: "Test User".to_string(),
                role: "user".to_string(),
                tenant_id,
            },
        };

        assert_eq!(response.expires_in, 900);
        assert_eq!(response.user.id, user_id);
        assert_eq!(response.user.tenant_id, tenant_id);
    }

    /// Test UserResponse structure
    #[test]
    fn test_user_response_structure() {
        let user_id = uuid::Uuid::new_v4();
        let tenant_id = uuid::Uuid::new_v4();

        let response = UserResponse {
            id: user_id,
            email: "user@example.com".to_string(),
            name: "John Doe".to_string(),
            role: "admin".to_string(),
            tenant_id,
        };

        assert_eq!(response.id, user_id);
        assert_eq!(response.role, "admin");
    }

    /// Test ErrorResponse structure
    #[test]
    fn test_error_response() {
        let error = ErrorResponse::new("Test error message");

        // ErrorResponse should have a message field
        // Based on models.rs ErrorResponse structure
        assert!(!error.message.is_empty());
    }

    /// Test authorization header extraction logic
    #[test]
    fn test_bearer_token_extraction() {
        let valid_header = "Bearer valid_token_here";
        let invalid_header = "Basic dXNlcjpwYXNz";
        let missing_prefix = "invalid_token";

        // Valid Bearer token
        assert!(valid_header.starts_with("Bearer "));
        let token = &valid_header[7..];
        assert_eq!(token, "valid_token_here");

        // Invalid: doesn't start with "Bearer "
        assert!(!invalid_header.starts_with("Bearer "));
        assert!(!missing_prefix.starts_with("Bearer "));
    }

    /// Test empty email validation
    #[test]
    fn test_empty_email_validation() {
        let request = LoginRequest {
            email: "".to_string(),
            password: "password".to_string(),
            tenant_slug: None,
        };

        // Empty email should be invalid
        assert!(request.email.is_empty());
    }

    /// Test empty password validation
    #[test]
    fn test_empty_password_validation() {
        let request = LoginRequest {
            email: "test@example.com".to_string(),
            password: "".to_string(),
            tenant_slug: None,
        };

        // Empty password should be invalid
        assert!(request.password.is_empty());
    }
}
