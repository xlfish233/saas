//! Rate limiting middleware using DashMap
//!
//! Supports tiered rate limits based on tenant subscription plans:
//! - Starter: 100 req/min
//! - Pro: 500 req/min
//! - Enterprise: 2000 req/min

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::tenant::Plan;

/// Rate limit entry
struct RateLimitEntry {
    count: u32,
    reset_at: Instant,
}

/// Tiered rate limit configuration per plan
#[derive(Clone, Debug)]
pub struct TieredRateLimits {
    /// Starter plan limit (requests per window)
    pub starter: u32,
    /// Pro plan limit (requests per window)
    pub pro: u32,
    /// Enterprise plan limit (requests per window)
    pub enterprise: u32,
    /// Time window duration
    pub window: Duration,
}

impl Default for TieredRateLimits {
    fn default() -> Self {
        Self {
            starter: 100,     // 100 req/min
            pro: 500,         // 500 req/min
            enterprise: 2000, // 2000 req/min
            window: Duration::from_secs(60),
        }
    }
}

impl TieredRateLimits {
    /// Get limit for a specific plan
    pub fn limit_for_plan(&self, plan: Plan) -> u32 {
        match plan {
            Plan::Starter => self.starter,
            Plan::Pro => self.pro,
            Plan::Enterprise => self.enterprise,
        }
    }
}

/// High-performance rate limiter using DashMap (lock-free)
/// Supports tiered limits based on tenant subscription plans
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<DashMap<String, RateLimitEntry>>,
    limits: TieredRateLimits,
}

impl RateLimiter {
    /// Create a new rate limiter with tiered limits
    pub fn new(limits: TieredRateLimits) -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
            limits,
        }
    }

    /// Create a rate limiter with default tiered limits
    pub fn with_defaults() -> Self {
        Self::new(TieredRateLimits::default())
    }

    /// Check if request is allowed for the given plan
    /// Returns Ok(remaining) with remaining requests count, or Err(retry_after_secs)
    pub fn check(&self, key: &str, plan: Plan) -> Result<u32, u64> {
        let now = Instant::now();
        let limit = self.limits.limit_for_plan(plan);

        let mut entry = self
            .requests
            .entry(key.to_string())
            .or_insert_with(|| RateLimitEntry {
                count: 0,
                reset_at: now + self.limits.window,
            });

        // Reset if window expired
        if now >= entry.reset_at {
            entry.count = 0;
            entry.reset_at = now + self.limits.window;
        }

        if entry.count >= limit {
            let retry_after = (entry.reset_at - now).as_secs();
            return Err(retry_after);
        }

        entry.count += 1;
        Ok(limit - entry.count)
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.requests.retain(|_, entry| now < entry.reset_at);
    }
}

/// Error response for rate limit exceeded
#[derive(Debug, Serialize)]
pub struct RateLimitErrorResponse {
    pub error: String,
    pub message: String,
    pub retry_after: u64,
}

/// Rate limit exceeded response
pub struct RateLimitExceeded {
    pub retry_after: u64,
}

impl IntoResponse for RateLimitExceeded {
    fn into_response(self) -> Response {
        let body = RateLimitErrorResponse {
            error: "rate_limit_exceeded".to_string(),
            message: "Too many requests. Please try again later.".to_string(),
            retry_after: self.retry_after,
        };

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                (header::RETRY_AFTER, self.retry_after.to_string()),
                (header::CONTENT_TYPE, "application/json".to_string()),
            ],
            axum::Json(body),
        )
            .into_response()
    }
}

/// Rate limiting middleware that supports tiered limits
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitExceeded> {
    // Build key from tenant_id + IP
    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous");

    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("unknown");

    // Get plan from header (default to Starter for unauthenticated requests)
    let plan = request
        .headers()
        .get("x-tenant-plan")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(Plan::Starter);

    let key = format!("{}:{}", tenant_id, ip);

    limiter
        .check(&key, plan)
        .map_err(|retry_after| RateLimitExceeded { retry_after })?;

    Ok(next.run(request).await)
}

/// Rate limiting middleware with exempt paths
/// Health and readiness endpoints are not rate limited
pub async fn rate_limit_with_exempt(
    axum::extract::State((limiter, exempt_paths)): axum::extract::State<(RateLimiter, Vec<String>)>,
    request: Request,
    next: Next,
) -> Result<Response, RateLimitExceeded> {
    let path = request.uri().path();

    // Check if path is exempt from rate limiting
    if exempt_paths
        .iter()
        .any(|exempt| path == exempt || path.starts_with(exempt))
    {
        return Ok(next.run(request).await);
    }

    // Build key from tenant_id + IP
    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous");

    let ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
        })
        .unwrap_or("unknown");

    // Get plan from header (default to Starter for unauthenticated requests)
    let plan = request
        .headers()
        .get("x-tenant-plan")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(Plan::Starter);

    let key = format!("{}:{}", tenant_id, ip);

    limiter
        .check(&key, plan)
        .map_err(|retry_after| RateLimitExceeded { retry_after })?;

    Ok(next.run(request).await)
}

/// Create a rate limiter with default tiered limits
pub fn default_rate_limiter() -> RateLimiter {
    RateLimiter::with_defaults()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tiered_limits() {
        let limits = TieredRateLimits::default();

        assert_eq!(limits.limit_for_plan(Plan::Starter), 100);
        assert_eq!(limits.limit_for_plan(Plan::Pro), 500);
        assert_eq!(limits.limit_for_plan(Plan::Enterprise), 2000);
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::with_defaults();

        for i in 0..99 {
            let result = limiter.check("test_key", Plan::Starter);
            assert!(result.is_ok(), "Request {} should be allowed", i);
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::with_defaults();

        // Exhaust the limit
        for _ in 0..100 {
            limiter.check("test_key", Plan::Starter).unwrap();
        }

        // Next request should be blocked
        let result = limiter.check("test_key", Plan::Starter);
        assert!(result.is_err());
        let retry_after = result.unwrap_err();
        assert!(retry_after > 0 && retry_after <= 60);
    }

    #[test]
    fn test_different_plans_have_different_limits() {
        let limiter = RateLimiter::with_defaults();

        // Exhaust Starter limit
        for _ in 0..100 {
            limiter.check("starter_key", Plan::Starter).unwrap();
        }
        assert!(limiter.check("starter_key", Plan::Starter).is_err());

        // Pro plan has higher limit
        assert!(limiter.check("pro_key", Plan::Pro).is_ok());

        // Enterprise plan has even higher limit
        for _ in 0..1999 {
            limiter.check("enterprise_key", Plan::Enterprise).unwrap();
        }
        assert!(limiter.check("enterprise_key", Plan::Enterprise).is_ok());
    }

    #[test]
    fn test_rate_limit_exceeded_response() {
        let exceeded = RateLimitExceeded { retry_after: 30 };
        let response = exceeded.into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let retry_after = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok());
        assert_eq!(retry_after, Some("30"));
    }
}
