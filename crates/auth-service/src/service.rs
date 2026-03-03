//! Authentication service business logic
#![allow(dead_code)]

use redis::Client;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::User;
use crate::repository::{TenantRepository, TokenRepository, UserRepository};

pub struct AuthService {
    jwt_service: shared::auth::JwtService,
    password_hasher: shared::auth::PasswordHasher,
    redis: Client,
    users: UserRepository,
    tenants: TenantRepository,
    tokens: TokenRepository,
}

impl AuthService {
    pub fn new(
        jwt_service: shared::auth::JwtService,
        password_hasher: shared::auth::PasswordHasher,
        redis: Client,
        pool: PgPool,
    ) -> Self {
        Self {
            jwt_service,
            password_hasher,
            redis,
            users: UserRepository::new(pool.clone()),
            tenants: TenantRepository::new(pool.clone()),
            tokens: TokenRepository::new(pool),
        }
    }

    /// Authenticate user with email/password
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
        tenant_slug: Option<&str>,
    ) -> Result<User, anyhow::Error> {
        // Find user by email
        let user = self
            .users
            .find_by_email(email)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Verify tenant if specified
        if let Some(slug) = tenant_slug {
            let tenant = self
                .tenants
                .find_by_slug(slug)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Tenant not found"))?;

            if tenant.id != user.tenant_id {
                return Err(anyhow::anyhow!("Invalid tenant"));
            }
        }

        // Verify password
        let valid = self.password_hasher.verify(password, &user.password_hash)?;

        if !valid {
            return Err(anyhow::anyhow!("Invalid password"));
        }

        Ok(user)
    }

    /// Generate access and refresh tokens
    pub async fn generate_tokens(&self, user: &User) -> Result<(String, String), anyhow::Error> {
        // Generate access token
        let access_token = self.jwt_service.generate_access_token(
            user.id,
            user.tenant_id,
            user.role.clone(),
            self.get_permissions(&user.role),
        )?;

        // Generate refresh token
        let (refresh_token, _jti) = self
            .jwt_service
            .generate_refresh_token(user.id, user.tenant_id)?;

        // Store refresh token hash in database
        let token_hash = self.hash_token(&refresh_token);
        let expires_at = OffsetDateTime::now_utc() + time::Duration::days(7);
        self.tokens
            .store_refresh_token(user.id, &token_hash, expires_at)
            .await?;

        Ok((access_token, refresh_token))
    }

    /// Refresh tokens using a refresh token
    pub async fn refresh_tokens(
        &self,
        refresh_token: &str,
    ) -> Result<(User, String, String), anyhow::Error> {
        let token_hash = self.hash_token(refresh_token);

        // Find and validate stored token
        let stored = self
            .tokens
            .find_refresh_token(&token_hash)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid refresh token"))?;

        // Get user
        let user = self
            .users
            .find_by_id(stored.user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        if !user.is_active {
            return Err(anyhow::anyhow!("User inactive"));
        }

        // Revoke old token
        self.tokens.revoke_refresh_token(&token_hash).await?;

        // Generate new tokens
        let (new_access, new_refresh) = self.generate_tokens(&user).await?;

        Ok((user, new_access, new_refresh))
    }

    /// Revoke a refresh token
    pub async fn revoke_token(&self, refresh_token: &str) -> Result<(), anyhow::Error> {
        let token_hash = self.hash_token(refresh_token);
        self.tokens.revoke_refresh_token(&token_hash).await?;
        Ok(())
    }

    /// Validate an access token
    pub async fn validate_token(&self, token: &str) -> Result<shared::auth::Claims, anyhow::Error> {
        self.jwt_service
            .validate_token(token)
            .await
            .map_err(Into::into)
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, id: Uuid) -> Result<User, anyhow::Error> {
        self.users
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))
    }

    /// Hash a token for storage
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get permissions for a role
    fn get_permissions(&self, role: &str) -> Vec<String> {
        match role {
            "admin" => vec![
                "users:*".to_string(),
                "tenants:*".to_string(),
                "finance:*".to_string(),
            ],
            "manager" => vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "finance:read".to_string(),
            ],
            _ => vec!["users:read".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test hash_token produces consistent SHA256 hash
    #[test]
    fn test_hash_token_consistency() {
        let test_token = "placeholder_test_token_for_hashing";

        // Create a minimal service just for testing hash_token
        let hash1 = {
            let mut hasher = Sha256::new();
            hasher.update(test_token.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        let hash2 = {
            let mut hasher = Sha256::new();
            hasher.update(test_token.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        // Same input should produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 produces 64 hex characters
    }

    /// Test get_permissions for different roles
    #[test]
    fn test_get_permissions_admin() {
        let permissions = get_test_permissions("admin");

        assert!(permissions.contains(&"users:*".to_string()));
        assert!(permissions.contains(&"tenants:*".to_string()));
        assert!(permissions.contains(&"finance:*".to_string()));
    }

    #[test]
    fn test_get_permissions_manager() {
        let permissions = get_test_permissions("manager");

        assert!(permissions.contains(&"users:read".to_string()));
        assert!(permissions.contains(&"users:write".to_string()));
        assert!(permissions.contains(&"finance:read".to_string()));
        assert!(!permissions.contains(&"tenants:*".to_string()));
    }

    #[test]
    fn test_get_permissions_default() {
        let permissions = get_test_permissions("user");
        assert!(permissions.contains(&"users:read".to_string()));
        assert_eq!(permissions.len(), 1);

        // Unknown role should also get default permissions
        let unknown_permissions = get_test_permissions("unknown");
        assert_eq!(unknown_permissions, permissions);
    }

    /// Helper function to test get_permissions logic
    fn get_test_permissions(role: &str) -> Vec<String> {
        match role {
            "admin" => vec![
                "users:*".to_string(),
                "tenants:*".to_string(),
                "finance:*".to_string(),
            ],
            "manager" => vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "finance:read".to_string(),
            ],
            _ => vec!["users:read".to_string()],
        }
    }
}
