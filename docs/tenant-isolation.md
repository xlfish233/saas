# 租户隔离策略

## 隔离层级

本平台支持三种租户隔离级别，满足不同客户的安全和合规需求：

```
┌─────────────────────────────────────────────────────────────────┐
│                        租户隔离层级                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Pool (共享池)          Bridge (桥接)         Silo (独享)       │
│  ┌─────────────┐       ┌─────────────┐       ┌─────────────┐   │
│  │  Database   │       │  Database   │       │  Database   │   │
│  │  ┌───────┐  │       │  ┌───────┐  │       │  ┌───────┐  │   │
│  │  │ T1    │  │       │  │ T1    │  │       │  │  T1   │  │   │
│  │  │ T2    │  │       │  ├───────┤  │       │  └───────┘  │   │
│  │  │ T3    │  │       │  │ T2    │  │       │             │   │
│  │  └───────┘  │       │  └───────┘  │       │  独立实例    │   │
│  │  共享表     │       │  独立Schema │       │  完全隔离    │   │
│  └─────────────┘       └─────────────┘       └─────────────┘   │
│                                                                 │
│  适用: Starter          适用: Pro              适用: Enterprise │
│  成本: 最低             成本: 中等             成本: 最高        │
│  隔离: 逻辑             隔离: Schema           隔离: 物理        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 隔离级别详情

### Level 1: Pool (共享池)

**适用场景**: Starter 套餐，小型企业

**特点**:
- 所有租户共享同一数据库和表
- 通过 `tenant_id` 列实现逻辑隔离
- 行级安全策略 (RLS) 强制隔离
- 成本最低，密度最高

**实现**:

```sql
-- 所有表包含 tenant_id
CREATE TABLE orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    customer_id UUID NOT NULL,
    total DECIMAL(12,2) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 创建行级安全策略
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON orders
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

-- 应用层设置租户上下文
SET app.current_tenant = 'tenant-uuid-here';
```

### Level 2: Bridge (桥接)

**适用场景**: Pro 套餐，中型企业

**特点**:
- 每个租户拥有独立 Schema
- 同一数据库实例，不同 Schema
- 更强的数据隔离
- 支持租户级备份/恢复

**实现**:

```sql
-- 租户专属 Schema
CREATE SCHEMA tenant_001;

-- 在租户 Schema 中创建表
CREATE TABLE tenant_001.orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    customer_id UUID NOT NULL,
    total DECIMAL(12,2) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- 连接时设置 search_path
SET search_path TO tenant_001, public;
```

**租户配置**:

```yaml
# 租户路由配置
tenant_config:
  tenant_001:
    isolation: bridge
    schema: tenant_001
    database: erp_pool_1
  tenant_002:
    isolation: bridge
    schema: tenant_002
    database: erp_pool_1
```

### Level 3: Silo (独享)

**适用场景**: Enterprise 套餐，大型企业，合规要求

**特点**:
- 每个租户独立数据库实例
- 完全物理隔离
- 独立计算/存储资源
- 支持自定义加密密钥
- 满足最严格合规要求

**实现**:

```yaml
# 租户独享配置
tenant_config:
  tenant_enterprise_001:
    isolation: silo
    database:
      endpoint: tenant-enterprise-001.cluster-xxx.us-east-1.rds.amazonaws.com
      database: erp
      credentials_secret: arn:aws:secretsmanager:xxx:tenant-enterprise-001-db
    encryption:
      kms_key: arn:aws:kms:xxx:tenant-enterprise-001-key
```

## 路由逻辑

```rust
// crates/shared/src/tenant/router.rs
pub struct TenantRouter {
    pool_tenants: SqlitePool,           // 共享池连接
    bridge_pools: HashMap<Uuid, PgPool>, // Bridge 连接池
    silo_pools: HashMap<Uuid, PgPool>,   // Silo 连接池
}

impl TenantRouter {
    pub async fn get_connection(&self, tenant_id: Uuid) -> Result<TenantConnection, Error> {
        let tenant = self.get_tenant_config(tenant_id).await?;
        
        match tenant.isolation_level {
            IsolationLevel::Pool => {
                // 使用共享连接，设置 tenant_id 上下文
                let conn = self.pool_tenants.acquire().await?;
                conn.execute(&format!("SET app.current_tenant = '{}'", tenant_id)).await?;
                Ok(TenantConnection::Pool(conn))
            }
            IsolationLevel::Bridge => {
                // 切换到租户 Schema
                let conn = self.bridge_pools.get(&tenant_id)
                    .ok_or(Error::TenantNotFound)?
                    .acquire().await?;
                conn.execute(&format!("SET search_path TO {}", tenant.schema)).await?;
                Ok(TenantConnection::Bridge(conn))
            }
            IsolationLevel::Silo => {
                // 连接到租户专属数据库
                let conn = self.silo_pools.get(&tenant_id)
                    .ok_or(Error::TenantNotFound)?
                    .acquire().await?;
                Ok(TenantConnection::Silo(conn))
            }
        }
    }
}
```

## Kubernetes 资源隔离

### Pool 租户

```yaml
# 共享命名空间，通过标签区分
apiVersion: v1
kind: Namespace
metadata:
  name: erp-tenants-pool
  labels:
    isolation: pool
---
# ResourceQuota 按租户限制
apiVersion: v1
kind: ResourceQuota
metadata:
  name: tenant-001-quota
  namespace: erp-tenants-pool
spec:
  hard:
    requests.cpu: "2"
    requests.memory: 4Gi
    limits.cpu: "4"
    limits.memory: 8Gi
```

### Silo 租户

```yaml
# 独立命名空间
apiVersion: v1
kind: Namespace
metadata:
  name: tenant-enterprise-001
  labels:
    tenant-id: enterprise-001
    isolation: silo
    pod-security.kubernetes.io/enforce: restricted
---
# NetworkPolicy 严格隔离
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: deny-all-ingress
  namespace: tenant-enterprise-001
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
  ingress: []  # 默认拒绝所有入站
  egress: []   # 默认拒绝所有出站
```

## 资源配额

| 级别 | CPU | 内存 | 存储 | 数据库连接 |
|-----|-----|-----|-----|-----------|
| Pool | 0.5-2 | 1-4GB | 10GB | 共享池 |
| Bridge | 2-8 | 4-16GB | 50GB | 专用 Schema |
| Silo | 8-32+ | 16-64GB+ | 500GB+ | 独立实例 |

## 迁移策略

租户可在不同隔离级别间迁移：

```
Pool → Bridge: 导出数据 → 创建 Schema → 导入数据
Bridge → Silo: 导出数据 → 创建数据库 → 导入数据 + DNS 切换
Silo → Bridge: 导出数据 → 删除实例 → 导入 Schema
```

迁移服务提供零停机迁移能力。
