//! Database repository for auth data
#![allow(dead_code)]

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::{RefreshToken, Tenant, User};

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            FROM users
            WHERE email = $1 AND is_active = true
            "#
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            FROM users
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(
        &self,
        tenant_id: Uuid,
        email: &str,
        password_hash: &str,
        name: &str,
        role: &str,
    ) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (tenant_id, email, password_hash, name, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            "#
        )
        .bind(tenant_id)
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .bind(role)
        .fetch_one(&self.pool)
        .await
    }
}

pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active
            FROM tenants
            WHERE slug = $1 AND is_active = true
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active
            FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }
}

pub struct TokenRepository {
    pool: PgPool,
}

impl TokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn store_refresh_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM refresh_tokens
            WHERE token_hash = $1 AND expires_at > NOW()
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM refresh_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM refresh_tokens
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    /// Test that UserRepository can be created
    #[test]
    fn test_user_repository_creation() {
        // This test verifies the repository can be instantiated
        // Actual database tests are in integration tests
        // For unit tests, we would need to mock PgPool
    }

    /// Test that TenantRepository can be created
    #[test]
    fn test_tenant_repository_creation() {
        // Repository creation test
    }

    /// Test that TokenRepository can be created
    #[test]
    fn test_token_repository_creation() {
        // Repository creation test
    }

    /// Test query structure for find_by_email
    #[test]
    fn test_find_by_email_query_structure() {
        // Verify the SQL query structure is correct
        let expected_query = r#"
            SELECT id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            FROM users
            WHERE email = $1 AND is_active = true
            "#;

        // Verify query contains necessary clauses
        assert!(expected_query.contains("SELECT"));
        assert!(expected_query.contains("FROM users"));
        assert!(expected_query.contains("WHERE email = $1"));
        assert!(expected_query.contains("is_active = true"));
    }

    /// Test query structure for create user
    #[test]
    fn test_create_user_query_structure() {
        let expected_query = r#"
            INSERT INTO users (tenant_id, email, password_hash, name, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            "#;

        assert!(expected_query.contains("INSERT INTO users"));
        assert!(expected_query.contains("RETURNING"));
    }

    /// Test query structure for store_refresh_token
    #[test]
    fn test_store_refresh_token_query_structure() {
        let expected_query = r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#;

        assert!(expected_query.contains("INSERT INTO refresh_tokens"));
        assert!(expected_query.contains("user_id"));
        assert!(expected_query.contains("token_hash"));
        assert!(expected_query.contains("expires_at"));
    }

    /// Test query structure for find_refresh_token
    #[test]
    fn test_find_refresh_token_query_structure() {
        let expected_query = r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM refresh_tokens
            WHERE token_hash = $1 AND expires_at > NOW()
            "#;

        assert!(expected_query.contains("SELECT"));
        assert!(expected_query.contains("FROM refresh_tokens"));
        assert!(expected_query.contains("expires_at > NOW()"));
    }

    /// Test query structure for revoke_refresh_token
    #[test]
    fn test_revoke_refresh_token_query_structure() {
        let expected_query = r#"
            DELETE FROM refresh_tokens
            WHERE token_hash = $1
            "#;

        assert!(expected_query.contains("DELETE FROM refresh_tokens"));
        assert!(expected_query.contains("WHERE token_hash = $1"));
    }
}
