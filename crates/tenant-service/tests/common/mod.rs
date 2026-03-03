//! Test infrastructure and utilities for tenant-service integration tests
//!
//! This module provides:
//! - Testcontainers setup for PostgreSQL
//! - Schema isolation testing utilities
//! - Test fixtures and factories

#![allow(dead_code)]

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// Setup test database with testcontainers
pub async fn setup_test_db() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    let container = Postgres::default()
        .with_user("testuser")
        .with_password("placeholder")
        .with_db_name("test_db")
        .start()
        .await
        .expect("Failed to start Postgres container");

    let connection_string = format!(
        "postgres://testuser:placeholder@127.0.0.1:{}/test_db",
        container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get port")
    );

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&connection_string)
        .await
        .expect("Failed to connect to test database");

    run_migrations(&pool).await;

    (pool, container)
}

/// Run database migrations
async fn run_migrations(pool: &PgPool) {
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
        );

        CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
        CREATE INDEX IF NOT EXISTS idx_tenants_is_active ON tenants(is_active);
        "#,
    )
    .execute(pool)
    .await
    .expect("Failed to run migrations");
}

/// Helper to create a test tenant
pub async fn create_test_tenant(
    pool: &PgPool,
    name: &str,
    slug: &str,
    isolation_level: &str,
    plan: &str,
) -> Uuid {
    let tenant_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, name, slug, isolation_level, plan, is_active)
        VALUES ($1, $2, $3, $4, $5, true)
        "#,
    )
    .bind(tenant_id)
    .bind(name)
    .bind(slug)
    .bind(isolation_level)
    .bind(plan)
    .execute(pool)
    .await
    .expect("Failed to create test tenant");

    tenant_id
}

/// Helper to check if a schema exists
pub async fn schema_exists(pool: &PgPool, schema_name: &str) -> bool {
    let result: Option<(bool,)> = sqlx::query_as(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM information_schema.schemata
            WHERE schema_name = $1
        )
        "#,
    )
    .bind(schema_name)
    .fetch_optional(pool)
    .await
    .expect("Failed to check schema existence");

    result.map(|(exists,)| exists).unwrap_or(false)
}

/// Helper to clean up test data and schemas
pub async fn cleanup_test_data(pool: &PgPool) {
    // Get all tenant schemas
    let schemas: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT schema_name FROM tenants
        WHERE schema_name IS NOT NULL
        "#,
    )
    .fetch_all(pool)
    .await
    .expect("Failed to fetch tenant schemas");

    // Drop all tenant schemas
    for (schema_name,) in schemas {
        sqlx::query(&format!("DROP SCHEMA IF EXISTS {} CASCADE", schema_name))
            .execute(pool)
            .await
            .ok();
    }

    // Truncate tenants table
    sqlx::query("TRUNCATE TABLE tenants CASCADE")
        .execute(pool)
        .await
        .expect("Failed to cleanup test data");
}

/// Helper to create a tenant schema manually for testing
pub async fn create_schema(pool: &PgPool, schema_name: &str) {
    sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name))
        .execute(pool)
        .await
        .expect("Failed to create schema");
}
