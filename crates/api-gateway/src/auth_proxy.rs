//! Auth proxy service - forwards requests to auth-service

use axum::{http::StatusCode, response::IntoResponse, Json};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

/// Auth proxy service for forwarding requests to auth-service
#[derive(Debug, Clone)]
pub struct AuthProxy {
    client: Client,
    base_url: String,
}

// ============ Request DTOs ============

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub tenant_slug: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub tenant_id: uuid::Uuid,
    #[serde(default = "default_role")]
    pub role: String,
}

#[allow(dead_code)]
fn default_role() -> String {
    "user".to_string()
}

#[derive(Debug, Serialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

// ============ Response DTOs ============

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize)]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub tenant_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    #[allow(dead_code)]
    pub error: String,
    pub message: String,
}

impl AuthProxy {
    /// Create a new auth proxy
    pub fn new(client: Client, base_url: String) -> Self {
        Self { client, base_url }
    }

    /// Proxy login request to auth-service
    pub async fn login(&self, req: &LoginRequest) -> Result<AuthResponse, ProxyError> {
        self.post("/auth/login", req).await
    }

    /// Proxy register request to auth-service
    pub async fn register(&self, req: &RegisterRequest) -> Result<AuthResponse, ProxyError> {
        self.post("/auth/register", req).await
    }

    /// Proxy refresh token request to auth-service
    pub async fn refresh(&self, req: &RefreshRequest) -> Result<AuthResponse, ProxyError> {
        self.post("/auth/refresh", req).await
    }

    /// Proxy logout request to auth-service
    pub async fn logout(&self, req: &LogoutRequest) -> Result<(), ProxyError> {
        self.post_no_response("/auth/logout", req).await
    }

    /// Get current user info (requires forwarding Authorization header)
    pub async fn me(&self, auth_header: &str) -> Result<UserResponse, ProxyError> {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, "/auth/me"))
            .header("Authorization", auth_header)
            .send()
            .await?;

        self.handle_response(response).await
    }

    // ============ Private helpers ============

    async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, ProxyError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn post_no_response<T: Serialize>(&self, path: &str, body: &T) -> Result<(), ProxyError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(ProxyError::Auth(error.message))
        }
    }

    async fn handle_response<R: DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<R, ProxyError> {
        let status = response.status();
        if status.is_success() {
            Ok(response.json().await?)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(ProxyError::Auth(error.message))
        }
    }
}

// ============ Error handling ============

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Auth error: {0}")]
    Auth(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> axum::response::Response {
        use serde_json::json;

        let (status, message) = match &self {
            ProxyError::Request(_) => (StatusCode::BAD_GATEWAY, "Auth service unavailable"),
            ProxyError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.as_str()),
        };

        (
            status,
            Json(json!({
                "error": "auth_error",
                "message": message
            })),
        )
            .into_response()
    }
}
