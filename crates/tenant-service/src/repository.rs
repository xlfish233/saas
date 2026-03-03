//! Database repository for tenant data
#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Tenant;

pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_all(&self) -> Result<Vec<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active, schema_name, database_url, created_at, updated_at
            FROM tenants
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active, schema_name, database_url, created_at, updated_at
            FROM tenants
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            SELECT id, name, slug, isolation_level, plan, is_active, schema_name, database_url, created_at, updated_at
            FROM tenants
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(
        &self,
        name: &str,
        slug: &str,
        isolation_level: &str,
        plan: &str,
    ) -> Result<Tenant, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO tenants (name, slug, isolation_level, plan)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, slug, isolation_level, plan, is_active, schema_name, database_url, created_at, updated_at
            "#,
        )
        .bind(name)
        .bind(slug)
        .bind(isolation_level)
        .bind(plan)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: Option<&str>,
        plan: Option<&str>,
        is_active: Option<bool>,
        schema_name: Option<&str>,
    ) -> Result<Tenant, sqlx::Error> {
        sqlx::query_as::<_, Tenant>(
            r#"
            UPDATE tenants
            SET
                name = COALESCE($2, name),
                plan = COALESCE($3, plan),
                is_active = COALESCE($4, is_active),
                schema_name = COALESCE($5, schema_name),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, slug, isolation_level, plan, is_active, schema_name, database_url, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(plan)
        .bind(is_active)
        .bind(schema_name)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn soft_delete(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE tenants
            SET is_active = false, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn schema_exists(&self, schema_name: &str) -> Result<bool, sqlx::Error> {
        let result: Option<(bool,)> = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = $1)",
        )
        .bind(schema_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|(b,)| b).unwrap_or(false))
    }

    pub async fn create_schema(&self, schema_name: &str) -> Result<(), sqlx::Error> {
        // 验证 schema 名称只包含安全字符 (DDL 不支持参数化)
        if !schema_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(sqlx::Error::Protocol(
                "Invalid schema name: must contain only alphanumeric characters and underscores"
                    .into(),
            ));
        }
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
