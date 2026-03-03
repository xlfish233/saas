//! Authentication middleware

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::auth::{Claims, JwtService};

/// Authentication middleware
pub async fn auth_middleware(
    axum::extract::State(jwt_service): axum::extract::State<JwtService>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    let claims = jwt_service
        .validate_token(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Store claims in request extensions
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Extract claims from request
pub fn get_claims(request: &Request) -> Option<&Claims> {
    request.extensions().get::<Claims>()
}
