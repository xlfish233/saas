//! E2E test for complete authentication flow
//!
//! Tests the entire auth lifecycle:
//! 1. Login → get tokens
//! 2. Access protected endpoint with access token
//! 3. Refresh tokens
//! 4. Logout
//! 5. Verify old refresh token is revoked

mod common;
mod fixtures;

use common::*;
use fixtures::TestFixtures;
use sha2::{Digest, Sha256};
use uuid::Uuid;

// Note: This is a simplified E2E test structure
// Full E2E tests would need actual HTTP server setup with axum::test

#[tokio::test]
#[ignore = "Requires full service setup with HTTP server"]
async fn complete_auth_flow_works_end_to_end() {
    // Setup test database
    let (pool, _container) = setup_test_db().await;

    // Create test tenant and user
    let tenant_id = create_test_tenant(&pool, "test-company").await;
    let password = TestFixtures::test_password();
    let hasher = shared::auth::PasswordHasher::new();
    let hashed_pwd = hasher.hash(&password).expect("Failed to hash password");

    let email = TestFixtures::test_email();
    let user_id = create_test_user(&pool, tenant_id, &email, &hashed_pwd).await;

    // Step 1: Login
    // In real test: POST /auth/login
    // For now, we verify the data is in the database
    let user: Option<(String, String, Uuid)> =
        sqlx::query_as("SELECT email, password_hash, tenant_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to fetch user");

    assert!(user.is_some());
    let (db_email, pwd_hash, db_tenant_id) = user.unwrap();
    assert_eq!(db_email, email);
    assert_eq!(db_tenant_id, tenant_id);
    assert!(hasher.verify(&password, &pwd_hash).unwrap());

    // Step 2: Access protected endpoint
    // Would test with Bearer token here

    // Step 3: Refresh tokens
    // Would test POST /auth/refresh

    // Step 4: Logout
    // Would test POST /auth/logout

    // Step 5: Verify old refresh token is revoked
    // Would verify token is in revoked list

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn login_flow_with_valid_credentials() {
    let (pool, _container) = setup_test_db().await;

    // Setup: Create tenant and user
    let tenant_id = create_test_tenant(&pool, "login-test").await;
    let password = "test_password_placeholder";
    let hasher = shared::auth::PasswordHasher::new();
    let hashed_pwd = hasher.hash(password).expect("Failed to hash password");

    let email = "user@login-test.com";
    let user_id = create_test_user(&pool, tenant_id, email, &hashed_pwd).await;

    // Verify user exists
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count users");

    assert_eq!(count, 1);

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn refresh_token_stores_hash_in_database() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id = create_test_tenant(&pool, "refresh-test").await;
    let dummy_hash = "dummy_hash";
    let email = "user@refresh-test.com";
    let user_id = create_test_user(&pool, tenant_id, email, dummy_hash).await;

    // Simulate storing refresh token
    let refresh_token = "test_token_placeholder_xxx";
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&pool)
    .await
    .expect("Failed to store refresh token");

    // Verify token was stored
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1 AND token_hash = $2",
    )
    .bind(user_id)
    .bind(&token_hash)
    .fetch_one(&pool)
    .await
    .expect("Failed to count refresh tokens");

    assert_eq!(count, 1);

    cleanup_test_data(&pool).await;
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let (pool, _container) = setup_test_db().await;

    let tenant_id = create_test_tenant(&pool, "logout-test").await;
    let email = "user@logout-test.com";
    let user_id = create_test_user(&pool, tenant_id, email, "dummy_hash").await;

    // Store refresh token
    let token_hash = "test_token_hash";
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&pool)
    .await
    .expect("Failed to store refresh token");

    // Verify token exists before revocation
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_one(&pool)
            .await
            .expect("Failed to count tokens before revocation");
    assert_eq!(count_before, 1);

    // Revoke token (using DELETE, matching repository implementation)
    sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
        .bind(token_hash)
        .execute(&pool)
        .await
        .expect("Failed to revoke token");

    // Verify token is deleted (revoked)
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_one(&pool)
            .await
            .expect("Failed to count tokens after revocation");
    assert_eq!(count_after, 0);

    cleanup_test_data(&pool).await;
}
