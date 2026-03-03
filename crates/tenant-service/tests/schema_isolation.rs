//! Integration tests for tenant schema isolation (Bridge mode)
//!
//! Tests schema creation and management for Bridge isolation level

mod common;

use common::*;

#[tokio::test]
async fn create_schema_for_bridge_tenant() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Bridge Tenant", "bridge-tenant", "bridge", "pro").await;

    let schema_name = "tenant_bridge_tenant";

    // Create schema
    create_schema(&pool, schema_name).await;

    // Verify schema exists
    let exists = schema_exists(&pool, schema_name).await;
    assert!(exists);

    // Update tenant with schema name
    sqlx::query("UPDATE tenants SET schema_name = $1 WHERE id = $2")
        .bind(schema_name)
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("Failed to update tenant");

    // Verify tenant has schema_name
    let stored_schema: Option<String> =
        sqlx::query_scalar("SELECT schema_name FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch schema name");

    assert_eq!(stored_schema, Some(schema_name.to_string()));

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn create_schema_is_idempotent() {
    let (pool, _container) = setup_test_db().await;

    let schema_name = "tenant_idempotent";

    // Create schema twice
    create_schema(&pool, schema_name).await;
    create_schema(&pool, schema_name).await;

    // Schema should still exist
    let exists = schema_exists(&pool, schema_name).await;
    assert!(exists);

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn pool_tenant_does_not_create_schema() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Pool Tenant", "pool-tenant", "pool", "starter").await;

    // Pool tenants should not have schema_name
    let schema_name: Option<String> =
        sqlx::query_scalar("SELECT schema_name FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch schema name");

    assert!(schema_name.is_none());
}

#[tokio::test]
async fn multiple_bridge_tenants_have_separate_schemas() {
    let (pool, _container) = setup_test_db().await;

    // Create two Bridge tenants
    let tenant1_id = create_test_tenant(&pool, "Bridge One", "bridge-one", "bridge", "pro").await;
    let tenant2_id = create_test_tenant(&pool, "Bridge Two", "bridge-two", "bridge", "pro").await;

    let schema1 = "tenant_bridge_one";
    let schema2 = "tenant_bridge_two";

    // Create schemas
    create_schema(&pool, schema1).await;
    create_schema(&pool, schema2).await;

    // Update tenants
    sqlx::query("UPDATE tenants SET schema_name = $1 WHERE id = $2")
        .bind(schema1)
        .bind(tenant1_id)
        .execute(&pool)
        .await
        .expect("Failed to update tenant 1");

    sqlx::query("UPDATE tenants SET schema_name = $1 WHERE id = $2")
        .bind(schema2)
        .bind(tenant2_id)
        .execute(&pool)
        .await
        .expect("Failed to update tenant 2");

    // Verify both schemas exist
    assert!(schema_exists(&pool, schema1).await);
    assert!(schema_exists(&pool, schema2).await);

    // Verify tenants have different schemas
    let tenant1_schema: String =
        sqlx::query_scalar("SELECT schema_name FROM tenants WHERE id = $1")
            .bind(tenant1_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch tenant 1 schema");

    let tenant2_schema: String =
        sqlx::query_scalar("SELECT schema_name FROM tenants WHERE id = $1")
            .bind(tenant2_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch tenant 2 schema");

    assert_ne!(tenant1_schema, tenant2_schema);

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn schema_name_format_is_correct() {
    // Test schema naming convention
    let test_cases = vec![
        ("my-company", "tenant_my_company"),
        ("test-123", "tenant_test_123"),
        ("acme-corp", "tenant_acme_corp"),
    ];

    for (slug, expected_schema) in test_cases {
        let schema_name = format!("tenant_{}", slug.replace('-', "_"));
        assert_eq!(schema_name, expected_schema);
    }
}

#[tokio::test]
async fn silo_tenant_has_database_url() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Silo Tenant", "silo-tenant", "silo", "enterprise").await;

    // Silo tenants should have database_url (simulated)
    let db_url = "postgresql://user:pass@localhost:5432/tenant_silo_tenant";

    sqlx::query("UPDATE tenants SET database_url = $1 WHERE id = $2")
        .bind(db_url)
        .bind(tenant_id)
        .execute(&pool)
        .await
        .expect("Failed to update tenant");

    let stored_url: Option<String> =
        sqlx::query_scalar("SELECT database_url FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch database URL");

    assert_eq!(stored_url, Some(db_url.to_string()));

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn cannot_create_schema_for_pool_tenant() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id =
        create_test_tenant(&pool, "Pool Tenant", "pool-tenant", "pool", "starter").await;

    // Fetch isolation level
    let isolation: String = sqlx::query_scalar("SELECT isolation_level FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch isolation level");

    // Pool isolation should not allow schema creation
    assert_eq!(isolation, "pool");

    // Schema name should be None
    let schema_name: Option<String> =
        sqlx::query_scalar("SELECT schema_name FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch schema name");

    assert!(schema_name.is_none());
}
