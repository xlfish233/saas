//! Integration tests for tenant CRUD operations
//!
//! Tests tenant creation, retrieval, update, and deletion

mod common;

use common::*;
use uuid::Uuid;

#[tokio::test]
async fn create_tenant_success() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Test Company", "test-company", "pool", "starter").await;

    // Verify tenant was created
    let tenant: Option<(String, String, String, String, bool)> = sqlx::query_as(
        "SELECT name, slug, isolation_level, plan, is_active FROM tenants WHERE id = $1",
    )
    .bind(tenant_id)
    .fetch_optional(&pool)
    .await
    .expect("Failed to fetch tenant");

    assert!(tenant.is_some());
    let (name, slug, isolation, plan, is_active) = tenant.unwrap();
    assert_eq!(name, "Test Company");
    assert_eq!(slug, "test-company");
    assert_eq!(isolation, "pool");
    assert_eq!(plan, "starter");
    assert!(is_active);

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn create_tenant_duplicate_slug_fails() {
    let (pool, _container) = setup_test_db().await;

    let slug = "duplicate-slug";
    create_test_tenant(&pool, "First Company", slug, "pool", "starter").await;

    // Try to create second tenant with same slug
    let result = sqlx::query(
        r#"
        INSERT INTO tenants (id, name, slug, isolation_level, plan, is_active)
        VALUES (gen_random_uuid(), 'Second Company', $1, 'pool', 'starter', true)
        "#,
    )
    .bind(slug)
    .execute(&pool)
    .await;

    // Should fail due to unique constraint
    assert!(result.is_err());

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn get_tenant_by_id() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id = create_test_tenant(&pool, "Get Test", "get-test", "pool", "starter").await;

    // Fetch tenant
    let tenant: Option<(String, String)> =
        sqlx::query_as("SELECT name, slug FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to fetch tenant");

    assert!(tenant.is_some());
    let (name, slug) = tenant.unwrap();
    assert_eq!(name, "Get Test");
    assert_eq!(slug, "get-test");

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn get_nonexistent_tenant_returns_none() {
    let (pool, _container) = setup_test_db().await;

    let fake_id = Uuid::new_v4();

    let tenant: Option<(String,)> = sqlx::query_as("SELECT name FROM tenants WHERE id = $1")
        .bind(fake_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to fetch tenant");

    assert!(tenant.is_none());
}

#[tokio::test]
async fn update_tenant_plan() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Update Test", "update-test", "pool", "starter").await;

    // Update plan
    sqlx::query("UPDATE tenants SET plan = 'pro', updated_at = NOW() WHERE id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("Failed to update tenant");

    // Verify update
    let plan: String = sqlx::query_scalar("SELECT plan FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch plan");

    assert_eq!(plan, "pro");

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn update_tenant_name() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id = create_test_tenant(&pool, "Old Name", "old-name", "pool", "starter").await;

    // Update name
    sqlx::query("UPDATE tenants SET name = 'New Name', updated_at = NOW() WHERE id = $1")
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("Failed to update tenant");

    // Verify update
    let name: String = sqlx::query_scalar("SELECT name FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch name");

    assert_eq!(name, "New Name");

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn delete_tenant_soft_delete() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Delete Test", "delete-test", "pool", "starter").await;

    // Soft delete (set deleted_at and is_active = false)
    sqlx::query(
        r#"
        UPDATE tenants
        SET deleted_at = NOW(), is_active = false, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .execute(&pool)
    .await
    .expect("Failed to soft delete tenant");

    // Verify soft delete
    let result: Option<(bool, Option<chrono::DateTime<chrono::Utc>>)> =
        sqlx::query_as("SELECT is_active, deleted_at FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to fetch tenant");

    let (is_active, deleted_at) = result.expect("Tenant should still exist");
    assert!(!is_active);
    assert!(deleted_at.is_some());

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn list_active_tenants() {
    let (pool, _container) = setup_test_db().await;

    // Create multiple tenants
    create_test_tenant(&pool, "Active 1", "active-1", "pool", "starter").await;
    create_test_tenant(&pool, "Active 2", "active-2", "pool", "pro").await;

    let deleted_id = create_test_tenant(&pool, "Deleted", "deleted", "pool", "starter").await;

    // Soft delete one tenant
    sqlx::query("UPDATE tenants SET deleted_at = NOW(), is_active = false WHERE id = $1")
        .bind(deleted_id)
        .execute(&pool)
        .await
        .expect("Failed to soft delete tenant");

    // List active tenants
    let active_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tenants WHERE is_active = true")
            .fetch_one(&pool)
            .await
            .expect("Failed to count active tenants");

    assert_eq!(active_count, 2);

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn tenant_isolation_levels() {
    let (pool, _container) = setup_test_db().await;

    // Create tenants with different isolation levels
    let pool_tenant =
        create_test_tenant(&pool, "Pool Tenant", "pool-tenant", "pool", "starter").await;
    let bridge_tenant =
        create_test_tenant(&pool, "Bridge Tenant", "bridge-tenant", "bridge", "pro").await;
    let silo_tenant =
        create_test_tenant(&pool, "Silo Tenant", "silo-tenant", "silo", "enterprise").await;

    // Verify isolation levels
    let pool_level: String =
        sqlx::query_scalar("SELECT isolation_level FROM tenants WHERE id = $1")
            .bind(pool_tenant)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch pool tenant");

    let bridge_level: String =
        sqlx::query_scalar("SELECT isolation_level FROM tenants WHERE id = $1")
            .bind(bridge_tenant)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch bridge tenant");

    let silo_level: String =
        sqlx::query_scalar("SELECT isolation_level FROM tenants WHERE id = $1")
            .bind(silo_tenant)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch silo tenant");

    assert_eq!(pool_level, "pool");
    assert_eq!(bridge_level, "bridge");
    assert_eq!(silo_level, "silo");

    cleanup_test_data(&pool).await;
}
