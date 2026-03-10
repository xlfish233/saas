//! Audit logging middleware
//!
//! Records API request metadata to the audit_logs table for compliance and security.
//!
//! Features:
//! - Async logging (non-blocking)
//! - Structured JSON format
//! - Request ID tracking
//! - Sensitive data masking
//! - Configurable exempt paths

use axum::{extract::Request, middleware::Next, response::Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Sensitive field names that should be masked in audit logs
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "password_hash",
    "token",
    "refresh_token",
    "secret",
    "api_key",
];

/// Audit log entry stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub changes: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Request metadata collected by the middleware
#[derive(Debug, Clone)]
pub struct RequestMetadata {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub tenant_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub started_at: DateTime<Utc>,
}

/// Response metadata added after request processing
#[derive(Debug, Clone)]
pub struct ResponseMetadata {
    pub status_code: u16,
    pub duration_ms: u64,
}

/// Combined audit record
#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub request: RequestMetadata,
    pub response: ResponseMetadata,
}

/// Audit log service that handles async database writes
#[derive(Clone)]
pub struct AuditLogService {
    tx: mpsc::Sender<AuditRecord>,
}

impl AuditLogService {
    /// Create a new audit log service with a database pool
    pub fn new(pool: sqlx::PgPool, buffer_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<AuditRecord>(buffer_size);

        // Spawn background task for writing audit logs
        tokio::spawn(async move {
            while let Some(record) = rx.recv().await {
                if let Err(e) = write_audit_log(&pool, record).await {
                    tracing::error!("Failed to write audit log: {}", e);
                }
            }
        });

        Self { tx }
    }

    /// Queue an audit record for async writing
    pub async fn log(&self, record: AuditRecord) {
        if let Err(e) = self.tx.send(record).await {
            tracing::error!("Failed to queue audit log: {}", e);
        }
    }
}

/// Write audit log to database
async fn write_audit_log(pool: &sqlx::PgPool, record: AuditRecord) -> Result<(), sqlx::Error> {
    let audit_log = record_to_audit_log(record);

    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            id, tenant_id, user_id, action, resource_type,
            resource_id, changes, ip_address, user_agent, status, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(audit_log.id)
    .bind(audit_log.tenant_id)
    .bind(audit_log.user_id)
    .bind(audit_log.action)
    .bind(audit_log.resource_type)
    .bind(audit_log.resource_id)
    .bind(audit_log.changes)
    .bind(audit_log.ip_address)
    .bind(audit_log.user_agent)
    .bind(audit_log.status)
    .bind(audit_log.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Convert audit record to audit log entry
fn record_to_audit_log(record: AuditRecord) -> AuditLog {
    let status = if record.response.status_code < 400 {
        "success"
    } else if record.response.status_code < 500 {
        "client_error"
    } else {
        "server_error"
    };

    // Build changes JSON
    let changes = serde_json::json!({
        "request_id": record.request.request_id,
        "method": record.request.method,
        "path": record.request.path,
        "status_code": record.response.status_code,
        "duration_ms": record.response.duration_ms,
    });

    AuditLog {
        id: Uuid::new_v4(),
        tenant_id: record.request.tenant_id,
        user_id: record.request.user_id,
        action: format!(
            "{} {}",
            record.request.method,
            extract_resource_type(&record.request.path)
        ),
        resource_type: extract_resource_type(&record.request.path),
        resource_id: extract_resource_id(&record.request.path),
        changes: Some(changes),
        ip_address: record.request.ip_address,
        user_agent: record.request.user_agent,
        status: status.to_string(),
        created_at: record.request.started_at,
    }
}

/// Extract resource type from path (e.g., "/api/v1/users" -> "users")
fn extract_resource_type(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Skip common prefixes like "api", "v1"
    for part in parts.iter().skip(2) {
        // Skip UUIDs
        if !is_uuid(part) {
            return part.to_string();
        }
    }

    "unknown".to_string()
}

/// Extract resource ID from path if present
fn extract_resource_id(path: &str) -> Option<Uuid> {
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    for part in parts.iter().rev() {
        if let Ok(uuid) = Uuid::parse_str(part) {
            return Some(uuid);
        }
    }

    None
}

/// Check if a string looks like a UUID
fn is_uuid(s: &str) -> bool {
    Uuid::parse_str(s).is_ok()
}

/// Extract IP address from request
fn extract_ip(request: &Request) -> Option<String> {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Mask sensitive fields in JSON value
pub fn mask_sensitive_data(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if SENSITIVE_FIELDS.contains(&key.to_lowercase().as_str()) {
                    *val = serde_json::Value::String("***REDACTED***".to_string());
                } else {
                    mask_sensitive_data(val);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for val in arr.iter_mut() {
                mask_sensitive_data(val);
            }
        }
        _ => {}
    }
}

/// Audit logging middleware that logs all requests
pub async fn audit_middleware(
    axum::extract::State(audit_service): axum::extract::State<AuditLogService>,
    request: Request,
    next: Next,
) -> Response {
    let started_at = Utc::now();

    // Extract request metadata
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let method = request.method().to_string();
    let path = request.uri().path().to_string();

    // Extract tenant ID from header or extension
    let tenant_id = request
        .headers()
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| {
            // Try to get from claims extension (if auth middleware ran before)
            request
                .extensions()
                .get::<crate::auth::Claims>()
                .map(|c| c.tenant_id)
        });

    // Extract user ID from claims extension (if auth middleware ran before)
    let user_id = request
        .extensions()
        .get::<crate::auth::Claims>()
        .map(|c| c.sub);

    let ip_address = extract_ip(&request);
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Process request
    let response = next.run(request).await;

    // Calculate duration
    let duration_ms = (Utc::now() - started_at).num_milliseconds() as u64;

    // Build audit record
    let record = AuditRecord {
        request: RequestMetadata {
            request_id,
            method,
            path,
            tenant_id,
            user_id,
            ip_address,
            user_agent,
            started_at,
        },
        response: ResponseMetadata {
            status_code: response.status().as_u16(),
            duration_ms,
        },
    };

    // Queue for async logging (non-blocking)
    audit_service.log(record).await;

    response
}

/// Audit logging middleware with exempt paths
/// Health and readiness endpoints are not logged
pub async fn audit_with_exempt(
    axum::extract::State((audit_service, exempt_paths)): axum::extract::State<(
        AuditLogService,
        Vec<String>,
    )>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();

    // Check if path is exempt from audit logging
    if exempt_paths
        .iter()
        .any(|exempt| path == exempt || path.starts_with(exempt))
    {
        return next.run(request).await;
    }

    audit_middleware(axum::extract::State(audit_service), request, next).await
}

/// Create default audit log service with a database pool
pub fn create_audit_service(pool: sqlx::PgPool) -> AuditLogService {
    // Buffer size of 1000 should be enough for most use cases
    AuditLogService::new(pool, 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resource_type() {
        assert_eq!(extract_resource_type("/api/v1/users"), "users");
        assert_eq!(
            extract_resource_type("/api/v1/tenants/123e4567-e89b-12d3-a456-426614174000"),
            "tenants"
        );
        assert_eq!(extract_resource_type("/api/v1/auth/login"), "auth");
        assert_eq!(extract_resource_type("/health"), "unknown");
    }

    #[test]
    fn test_extract_resource_id() {
        let uuid_str = "123e4567-e89b-12d3-a456-426614174000";
        let uuid = Uuid::parse_str(uuid_str).unwrap();

        assert_eq!(
            extract_resource_id(&format!("/api/v1/users/{}", uuid_str)),
            Some(uuid)
        );
        assert_eq!(extract_resource_id("/api/v1/users"), None);
    }

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("123e4567-e89b-12d3-a456-426614174000"));
        assert!(!is_uuid("users"));
        assert!(!is_uuid("123"));
    }

    #[test]
    fn test_mask_sensitive_data() {
        let mut json = serde_json::json!({
            "email": "test@example.com",
            "password": "secret123",
            "profile": {
                "name": "John",
                "token": "abc123"
            }
        });

        mask_sensitive_data(&mut json);

        assert_eq!(json["email"], "test@example.com");
        assert_eq!(json["password"], "***REDACTED***");
        assert_eq!(json["profile"]["name"], "John");
        assert_eq!(json["profile"]["token"], "***REDACTED***");
    }

    #[test]
    fn test_record_to_audit_log() {
        let record = AuditRecord {
            request: RequestMetadata {
                request_id: "test-123".to_string(),
                method: "POST".to_string(),
                path: "/api/v1/users".to_string(),
                tenant_id: Some(Uuid::new_v4()),
                user_id: Some(Uuid::new_v4()),
                ip_address: Some("192.168.1.1".to_string()),
                user_agent: Some("TestAgent".to_string()),
                started_at: Utc::now(),
            },
            response: ResponseMetadata {
                status_code: 201,
                duration_ms: 50,
            },
        };

        let audit_log = record_to_audit_log(record);

        assert_eq!(audit_log.action, "POST users");
        assert_eq!(audit_log.resource_type, "users");
        assert_eq!(audit_log.status, "success");
        assert!(audit_log.changes.is_some());
    }

    #[test]
    fn test_status_classification() {
        // Success
        let record = AuditRecord {
            request: RequestMetadata {
                request_id: "test".to_string(),
                method: "GET".to_string(),
                path: "/api/v1/users".to_string(),
                tenant_id: None,
                user_id: None,
                ip_address: None,
                user_agent: None,
                started_at: Utc::now(),
            },
            response: ResponseMetadata {
                status_code: 200,
                duration_ms: 10,
            },
        };
        assert_eq!(record_to_audit_log(record).status, "success");

        // Client error
        let record = AuditRecord {
            request: RequestMetadata {
                request_id: "test".to_string(),
                method: "GET".to_string(),
                path: "/api/v1/users".to_string(),
                tenant_id: None,
                user_id: None,
                ip_address: None,
                user_agent: None,
                started_at: Utc::now(),
            },
            response: ResponseMetadata {
                status_code: 404,
                duration_ms: 5,
            },
        };
        assert_eq!(record_to_audit_log(record).status, "client_error");

        // Server error
        let record = AuditRecord {
            request: RequestMetadata {
                request_id: "test".to_string(),
                method: "GET".to_string(),
                path: "/api/v1/users".to_string(),
                tenant_id: None,
                user_id: None,
                ip_address: None,
                user_agent: None,
                started_at: Utc::now(),
            },
            response: ResponseMetadata {
                status_code: 500,
                duration_ms: 100,
            },
        };
        assert_eq!(record_to_audit_log(record).status, "server_error");
    }
}
