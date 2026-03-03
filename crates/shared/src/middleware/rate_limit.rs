//! Rate limiting middleware using DashMap

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Rate limit entry
struct RateLimitEntry {
    count: u32,
    reset_at: Instant,
}

/// High-performance rate limiter using DashMap (lock-free)
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<DashMap<String, RateLimitEntry>>,
    limit: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
            limit,
            window,
        }
    }

    /// Check if request is allowed
    pub fn check(&self, key: &str) -> Result<(), StatusCode> {
        let now = Instant::now();

        let mut entry = self.requests.entry(key.to_string()).or_insert_with(|| RateLimitEntry {
            count: 0,
            reset_at: now + self.window,
        });

        // Reset if window expired
        if now >= entry.reset_at {
            entry.count = 0;
            entry.reset_at = now + self.window;
        }

        if entry.count >= self.limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        entry.count += 1;
        Ok(())
    }

    /// Clean up expired entries
    pub fn cleanup(&self) {
        let now = Instant::now();
        self.requests.retain(|_, entry| now < entry.reset_at);
    }
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<RateLimiter>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
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

    let key = format!("{}:{}", tenant_id, ip);

    limiter.check(&key)?;
    Ok(next.run(request).await)
}

/// Create a rate limiter with sensible defaults
pub fn default_rate_limiter() -> RateLimiter {
    RateLimiter::new(100, Duration::from_secs(60)) // 100 requests per minute
}
