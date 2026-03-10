//! Database repository for tenant data
#![allow(dead_code)]

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::models::{Tenant, TenantUsage};

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

impl TenantRepository {
    /// Get usage for a tenant within a time period
    pub async fn get_usage(
        &self,
        tenant_id: Uuid,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<Option<TenantUsage>, sqlx::Error> {
        sqlx::query_as::<_, TenantUsage>(
            r#"
            SELECT id, tenant_id, user_count, storage_used_bytes, api_calls_count,
                   storage_files_count, period_start, period_end, updated_at
            FROM tenant_usage
            WHERE tenant_id = $1 AND period_start = $2 AND period_end = $3
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_optional(&self.pool)
        .await
    }

    /// Get or create usage record for current period
    pub async fn get_or_create_usage(
        &self,
        tenant_id: Uuid,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<TenantUsage, sqlx::Error> {
        // Try to get existing
        if let Some(usage) = self.get_usage(tenant_id, period_start, period_end).await? {
            return Ok(usage);
        }

        // Create new record
        sqlx::query_as::<_, TenantUsage>(
            r#"
            INSERT INTO tenant_usage (tenant_id, period_start, period_end)
            VALUES ($1, $2, $3)
            RETURNING id, tenant_id, user_count, storage_used_bytes, api_calls_count,
                      storage_files_count, period_start, period_end, updated_at
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
    }

    /// Increment API calls count
    pub async fn increment_api_calls(
        &self,
        tenant_id: Uuid,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenant_usage (tenant_id, period_start, period_end, api_calls_count)
            VALUES ($1, $2, $3, 1)
            ON CONFLICT (tenant_id, period_start)
            DO UPDATE SET api_calls_count = tenant_usage.api_calls_count + 1
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Increment user count
    pub async fn increment_user_count(
        &self,
        tenant_id: Uuid,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenant_usage (tenant_id, period_start, period_end, user_count)
            VALUES ($1, $2, $3, 1)
            ON CONFLICT (tenant_id, period_start)
            DO UPDATE SET user_count = tenant_usage.user_count + 1
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Decrement user count
    pub async fn decrement_user_count(
        &self,
        tenant_id: Uuid,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenant_usage (tenant_id, period_start, period_end, user_count)
            VALUES ($1, $2, $3, 0)
            ON CONFLICT (tenant_id, period_start)
            DO UPDATE SET user_count = GREATEST(tenant_usage.user_count - 1, 0)
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update storage usage
    pub async fn update_storage_usage(
        &self,
        tenant_id: Uuid,
        bytes_delta: i64,
        files_delta: i64,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenant_usage (tenant_id, period_start, period_end, storage_used_bytes, storage_files_count)
            VALUES ($1, $2, $3, GREATEST($4, 0), GREATEST($5, 0))
            ON CONFLICT (tenant_id, period_start)
            DO UPDATE SET
                storage_used_bytes = GREATEST(tenant_usage.storage_used_bytes + $4, 0),
                storage_files_count = GREATEST(tenant_usage.storage_files_count + $5, 0)
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .bind(bytes_delta)
        .bind(files_delta)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Set user count directly (for sync operations)
    pub async fn set_user_count(
        &self,
        tenant_id: Uuid,
        count: i32,
        period_start: OffsetDateTime,
        period_end: OffsetDateTime,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tenant_usage (tenant_id, period_start, period_end, user_count)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (tenant_id, period_start)
            DO UPDATE SET user_count = $4
            "#,
        )
        .bind(tenant_id)
        .bind(period_start)
        .bind(period_end)
        .bind(count)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
