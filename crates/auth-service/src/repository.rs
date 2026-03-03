//! Database repository for auth data

use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::models::{User, Tenant, RefreshToken};

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            FROM users
            WHERE email = $1 AND is_active = true
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, tenant_id: Uuid, email: &str, password_hash: &str, name: &str, role: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (tenant_id, email, password_hash, name, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, tenant_id, email, password_hash, name, role, is_active, created_at, updated_at
            "#,
            tenant_id, email, password_hash, name, role
        )
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
        sqlx::query_as!(
            Tenant,
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active
            FROM tenants
            WHERE slug = $1 AND is_active = true
            "#,
            slug
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as!(
            Tenant,
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active
            FROM tenants
            WHERE id = $1
            "#,
            id
        )
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

    pub async fn store_refresh_token(&self, user_id: Uuid, token_hash: &str, expires_at: chrono::DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#,
            user_id, token_hash, expires_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_refresh_token(&self, token_hash: &str) -> Result<Option<RefreshToken>, sqlx::Error> {
        sqlx::query_as!(
            RefreshToken,
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM refresh_tokens
            WHERE token_hash = $1 AND expires_at > NOW()
            "#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn revoke_refresh_token(&self, token_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE token_hash = $1
            "#,
            token_hash
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            DELETE FROM refresh_tokens
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
