//! Test infrastructure and utilities for auth-service integration tests
//!
//! This module provides:
//! - Testcontainers setup for PostgreSQL and Redis
//! - Database migration helpers
//! - Test fixtures and factories

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;
use tokio::time::timeout;
use uuid::Uuid;

const CONTAINER_START_TIMEOUT: Duration = Duration::from_secs(60);
const DB_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Setup test database with testcontainers
///
/// Returns a connection pool and container handle.
/// The container will be cleaned up when the handle is dropped.
pub async fn setup_test_db() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    // Start PostgreSQL container with timeout
    let container = timeout(
        CONTAINER_START_TIMEOUT,
        Postgres::default()
            .with_user("testuser")
            .with_password("placeholder")
            .with_db_name("test_db")
            .start(),
    )
    .await
    .expect("Timeout waiting for Postgres container to start")
    .expect("Failed to start Postgres container");

    let connection_string = format!(
        "postgres://testuser:placeholder@127.0.0.1:{}/test_db",
        container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get port")
    );

    // Create connection pool with timeout
    let pool = timeout(
        DB_CONNECT_TIMEOUT,
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&connection_string),
    )
    .await
    .expect("Timeout connecting to test database")
    .expect("Failed to connect to test database");

    // Run migrations
    run_migrations(&pool).await;

    (pool, container)
}

/// Setup test Redis with testcontainers
#[allow(dead_code)]
pub async fn setup_test_redis() -> (redis::Client, testcontainers::ContainerAsync<Redis>) {
    let container = Redis::default()
        .start()
        .await
        .expect("Failed to start Redis container");

    let redis_url = format!(
        "redis://127.0.0.1:{}/",
        container
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get port")
    );

    let client = redis::Client::open(redis_url.as_str()).expect("Failed to create Redis client");

    (client, container)
}

/// Run database migrations
async fn run_migrations(pool: &PgPool) {
    // For now, we'll create tables manually
    // In production, use sqlx::migrate!() macro

    // Create pgcrypto extension for gen_random_uuid()
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .execute(pool)
        .await
        .expect("Failed to create pgcrypto extension");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tenants (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL,
            slug TEXT NOT NULL UNIQUE,
            isolation_level TEXT NOT NULL DEFAULT 'pool',
            plan TEXT NOT NULL DEFAULT 'starter',
            is_active BOOLEAN NOT NULL DEFAULT true,
            schema_name TEXT,
            database_url TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create tenants table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            tenant_id UUID NOT NULL REFERENCES tenants(id),
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create users table");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS refresh_tokens (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            token_hash TEXT NOT NULL UNIQUE,
            expires_at TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to create refresh_tokens table");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id)")
        .execute(pool)
        .await
        .expect("Failed to create idx_users_tenant_id");

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id)")
        .execute(pool)
        .await
        .expect("Failed to create idx_refresh_tokens_user_id");

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token_hash ON refresh_tokens(token_hash)",
    )
    .execute(pool)
    .await
    .expect("Failed to create idx_refresh_tokens_token_hash");
}

/// Helper to create a test user
pub async fn create_test_user(
    pool: &PgPool,
    tenant_id: Uuid,
    email: &str,
    password_hash: &str,
) -> Uuid {
    let user_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, role, tenant_id, is_active)
        VALUES ($1, $2, $3, 'Test User', 'user', $4, true)
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(password_hash)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

/// Helper to create a test tenant
pub async fn create_test_tenant(pool: &PgPool, slug: &str) -> Uuid {
    let tenant_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, name, slug, isolation_level, plan, is_active)
        VALUES ($1, 'Test Tenant', $2, 'pool', 'starter', true)
        "#,
    )
    .bind(tenant_id)
    .bind(slug)
    .execute(pool)
    .await
    .expect("Failed to create test tenant");

    tenant_id
}

/// Helper to clean up test data
pub async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE refresh_tokens, users, tenants CASCADE")
        .execute(pool)
        .await
        .expect("Failed to cleanup test data");
}
