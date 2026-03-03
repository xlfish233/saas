//! SQLx Runtime Query Shim
//!
//! This crate provides a thin wrapper around `sqlx-core` and `sqlx-postgres`
//! to enable runtime SQL queries without compile-time macro verification.
//!
//! ## Purpose
//! - Eliminates the need for `SQLX_OFFLINE=true` in CI
//! - Simplifies local development (no database connection required for compilation)
//! - Reduces compile times by avoiding macro expansion
//!
//! ## Trade-offs
//! - **No compile-time SQL verification**: SQL errors only detected at runtime
//! - **Manual FromRow implementations**: Required for each model struct
//!
//! ## Usage
//! ```ignore
//! use sqlx::{PgPool, query_as, FromRow, Row};
//!
//! // Manual FromRow implementation
//! struct User { id: Uuid, name: String }
//! impl<'r> FromRow<'r, PgRow> for User {
//!     fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
//!         Ok(Self { id: row.try_get("id")?, name: row.try_get("name")? })
//!     }
//! }
//!
//! let users = query_as::<_, User>("SELECT id, name FROM users")
//!     .fetch_all(&pool).await?;
//! ```

pub use sqlx_core::error::{Error, Result};
pub use sqlx_core::executor::Executor;
pub use sqlx_core::from_row::FromRow;
pub use sqlx_core::query::query;
pub use sqlx_core::query_as::query_as;
pub use sqlx_core::row::Row;
pub use sqlx_postgres::{PgPool, Postgres};

pub mod pool {
    pub use sqlx_core::pool::{Pool, PoolConnection};
}

pub mod postgres {
    pub use sqlx_postgres::{PgPoolOptions, PgRow};
}
