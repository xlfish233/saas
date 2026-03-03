//! Tenant database routing

use sqlx::{pool::PoolConnection, PgPool, Postgres};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{IsolationLevel, Tenant};

/// Tenant-specific database connection
pub enum TenantConnection {
    /// Pool isolation - shared connection with tenant_id context
    Pool(PoolConnection<Postgres>),
    /// Bridge isolation - connection with schema set
    Bridge(PoolConnection<Postgres>),
    /// Silo isolation - dedicated database connection
    Silo(PoolConnection<Postgres>),
}

/// Router for tenant database connections
pub struct TenantRouter {
    /// Shared pool for Pool isolation level
    shared_pool: PgPool,
    /// Cached pools for Bridge isolation (same database, different schema)
    bridge_pools: Arc<RwLock<HashMap<Uuid, PgPool>>>,
    /// Cached pools for Silo isolation (separate databases)
    silo_pools: Arc<RwLock<HashMap<Uuid, PgPool>>>,
}

impl TenantRouter {
    /// Create a new tenant router with a shared pool
    pub fn new(shared_pool: PgPool) -> Self {
        Self {
            shared_pool,
            bridge_pools: Arc::new(RwLock::new(HashMap::new())),
            silo_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a connection for a tenant
    pub async fn get_connection(&self, tenant: &Tenant) -> Result<TenantConnection, sqlx::Error> {
        match tenant.isolation_level {
            IsolationLevel::Pool => {
                let mut conn = self.shared_pool.acquire().await?;
                // Set tenant context for row-level security
                sqlx::query(&format!("SET app.current_tenant = '{}'", tenant.id))
                    .execute(&mut *conn)
                    .await?;
                Ok(TenantConnection::Pool(conn))
            }
            IsolationLevel::Bridge => {
                let schema = tenant.schema_name.as_ref().ok_or_else(|| {
                    sqlx::Error::Configuration("Missing schema_name for Bridge tenant".into())
                })?;

                // Check cache
                let pools = self.bridge_pools.read().await;
                if let Some(pool) = pools.get(&tenant.id) {
                    let mut conn = pool.acquire().await?;
                    sqlx::query(&format!("SET search_path TO {}, public", schema))
                        .execute(&mut *conn)
                        .await?;
                    return Ok(TenantConnection::Bridge(conn));
                }
                drop(pools);

                // Create new connection
                let mut conn = self.shared_pool.acquire().await?;
                sqlx::query(&format!("SET search_path TO {}, public", schema))
                    .execute(&mut *conn)
                    .await?;
                Ok(TenantConnection::Bridge(conn))
            }
            IsolationLevel::Silo => {
                let db_url = tenant.database_url.as_ref().ok_or_else(|| {
                    sqlx::Error::Configuration("Missing database_url for Silo tenant".into())
                })?;

                // Check cache
                let pools = self.silo_pools.read().await;
                if let Some(pool) = pools.get(&tenant.id) {
                    return Ok(TenantConnection::Silo(pool.acquire().await?));
                }
                drop(pools);

                // Create new pool and cache it
                let pool = PgPool::connect(db_url).await?;
                let mut pools = self.silo_pools.write().await;
                pools.insert(tenant.id, pool);

                let conn = pools.get(&tenant.id).unwrap().acquire().await?;
                Ok(TenantConnection::Silo(conn))
            }
        }
    }

    /// Register a Silo tenant's database pool
    pub async fn register_silo_tenant(
        &self,
        tenant_id: Uuid,
        database_url: &str,
    ) -> Result<(), sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        let mut pools = self.silo_pools.write().await;
        pools.insert(tenant_id, pool);
        Ok(())
    }

    /// Remove a tenant's cached pool
    pub async fn remove_tenant(&self, tenant_id: Uuid) {
        let mut bridge = self.bridge_pools.write().await;
        bridge.remove(&tenant_id);

        let mut silo = self.silo_pools.write().await;
        silo.remove(&tenant_id);
    }
}
