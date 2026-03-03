pub use sqlx_core::error::{Error, Result};
pub use sqlx_core::from_row::FromRow;
pub use sqlx_core::pool;
pub use sqlx_core::query::query;
pub use sqlx_core::query_as::query_as;
pub use sqlx_core::row::Row;
pub use sqlx_postgres::{PgPool, Postgres};

pub mod postgres {
    pub use sqlx_postgres::{PgPoolOptions, PgRow};
}
