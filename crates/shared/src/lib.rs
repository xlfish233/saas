//! Shared library for ERP SaaS platform
//!
//! Provides common utilities, middleware, and types used across all services.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod middleware;
pub mod telemetry;
pub mod tenant;

pub use auth::*;
pub use config::*;
pub use db::*;
pub use error::*;
pub use middleware::*;
pub use telemetry::*;
pub use tenant::*;

/// Common result type used across services
pub type Result<T> = std::result::Result<T, Error>;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{Error, Result};
    pub use serde::{Deserialize, Serialize};
    pub use sqlx::PgPool;
    pub use uuid::Uuid;
}
