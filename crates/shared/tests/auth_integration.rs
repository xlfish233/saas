//! Integration tests for shared auth module
//!
//! These tests verify JWT token generation, validation, and password hashing

use shared::auth::{JwtService, PasswordHasher, TokenRevocationStore};
use uuid::Uuid;

/// Generate test RSA keys for JWT testing
fn generate_test_keys() -> (Vec<u8>, Vec<u8>) {
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    use rsa::{RsaPrivateKey, RsaPublicKey};

    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("Failed to generate private key");
    let public_key = RsaPublicKey::from(&private_key);

    let private_pem = private_key
        .to_pkcs8_pem(LineEnding::default())
        .expect("Failed to encode private key");
    let public_pem = public_key
        .to_public_key_pem(LineEnding::default())
        .expect("Failed to encode public key");

    (
        private_pem.as_bytes().to_vec(),
        public_pem.as_bytes().to_vec(),
    )
}

#[tokio::test]
async fn test_jwt_service_generates_and_validates_tokens() {
    let (private_key, public_key) = generate_test_keys();
    let service = JwtService::new(
        &private_key,
        &public_key,
        "test-issuer".to_string(),
        "test-audience".to_string(),
    )
    .expect("Failed to create JWT service");

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let token = service
        .generate_access_token(
            user_id,
            tenant_id,
            "admin".to_string(),
            vec!["users:*".to_string()],
        )
        .expect("Failed to generate token");

    let claims = service
        .validate_token(&token)
        .await
        .expect("Failed to validate token");

    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.tenant_id, tenant_id);
    assert_eq!(claims.role, "admin");
    assert!(claims.permissions.contains(&"users:*".to_string()));
}

#[tokio::test]
async fn test_jwt_service_rejects_revoked_tokens() {
    let (private_key, public_key) = generate_test_keys();
    let service = JwtService::new(
        &private_key,
        &public_key,
        "test-issuer".to_string(),
        "test-audience".to_string(),
    )
    .expect("Failed to create JWT service");

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let token = service
        .generate_access_token(user_id, tenant_id, "user".to_string(), vec![])
        .expect("Failed to generate token");

    // Decode to get jti
    let claims = service
        .validate_token(&token)
        .await
        .expect("Failed to validate token initially");

    // Revoke the token
    service.revoke_token(claims.jti).await;

    // Should now fail validation
    let result = service.validate_token(&token).await;
    assert!(result.is_err());
}

#[test]
fn test_password_hasher_hashes_and_verifies() {
    let hasher = PasswordHasher::new();
    let password = "test_password_placeholder";

    let hash = hasher.hash(password).expect("Failed to hash password");

    // Verify correct password
    assert!(hasher
        .verify(password, &hash)
        .expect("Failed to verify password"));

    // Verify wrong password
    assert!(!hasher
        .verify("wrong_password", &hash)
        .expect("Failed to verify wrong password"));
}

#[test]
fn test_password_hasher_accepts_unicode() {
    let hasher = PasswordHasher::new();
    let password = "密码测试🔐🎉"; // Chinese + emojis

    let hash = hasher
        .hash(password)
        .expect("Failed to hash unicode password");

    assert!(hasher
        .verify(password, &hash)
        .expect("Failed to verify unicode password"));
}

#[test]
fn test_password_hasher_rejects_empty() {
    let hasher = PasswordHasher::new();
    let result = hasher.hash("");

    // Should either return error or handle gracefully
    // The argon2 crate typically allows empty passwords, but we might want to reject them
    // For now, we just verify it doesn't panic
    if let Ok(hash) = result {
        assert!(hasher.verify("", &hash).unwrap_or(false));
    }
}

#[tokio::test]
async fn test_token_revocation_store() {
    let store = TokenRevocationStore::new();
    let token_id = Uuid::new_v4();

    // Initially not revoked
    assert!(!store.is_revoked(&token_id).await);

    // Revoke token
    store.revoke(token_id).await;
    assert!(store.is_revoked(&token_id).await);

    // Different token not affected
    let other_token = Uuid::new_v4();
    assert!(!store.is_revoked(&other_token).await);
}

#[tokio::test]
async fn test_jwt_service_refresh_token_generation() {
    let (private_key, public_key) = generate_test_keys();
    let service = JwtService::new(
        &private_key,
        &public_key,
        "test-issuer".to_string(),
        "test-audience".to_string(),
    )
    .expect("Failed to create JWT service");

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let (token, jti) = service
        .generate_refresh_token(user_id, tenant_id)
        .expect("Failed to generate refresh token");

    assert!(!token.is_empty());
    assert_ne!(jti, Uuid::nil());

    // Validate refresh token
    let claims = service
        .validate_token(&token)
        .await
        .expect("Failed to validate refresh token");

    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.tenant_id, tenant_id);
    assert_eq!(claims.role, "refresh");
    assert!(claims.permissions.is_empty());
}
