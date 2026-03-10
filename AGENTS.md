# AGENTS.md

> AI 助手开发指南 - 本文件为 AI 助手（如 Droid、Claude、GPT-4）提供项目上下文和开发规范。

## 项目概述

**ERP SaaS Platform** - 云原生多租户 ERP SaaS 平台，使用 Rust 构建的高性能后端服务。

### 当前阶段
- **优先级**: 本地开发优先，暂不部署到 AWS
- **状态**: 核心认证模块已完成并通过 e2e 测试，租户管理服务开发中

### 开发进度
**已完成模块**:
- ✅ **auth-service** - 用户认证服务 (已完成并通过 e2e 测试)
  - 用户注册/登录
  - JWT 令牌管理 (access + refresh tokens)
  - 密码哈希验证 (Argon2)
  - Token 刷新机制
  - 登出与会话管理
  - 23 个单元测试 + 3 个集成测试

- ✅ **api-gateway** - API 网关服务 (基础框架完成)
  - Auth Proxy 中间件 (代理认证请求到 auth-service)
  - 健康检查端点
  - CORS 配置

- ✅ **shared** - 共享库 (核心功能完成)
  - 配置管理
  - JWT 工具
  - 密码哈希工具
  - 数据库连接池
  - 遥测与日志
  - 7 个单元测试 + 7 个集成测试

- ✅ **tenant-service** - 租户管理服务 (基础框架完成)
  - 租户 CRUD 操作
  - Schema 隔离支持
  - 6 个单元测试 + 16 个集成测试

- ✅ **db-migrator** - 数据库迁移工具 (完成)
  - 迁移版本管理
  - 启动时自动迁移/验证
  - 3 个单元测试

**开发中**:
- 🚧 **api-gateway** - 完善网关功能
  - 请求路由与负载均衡
  - 限流与熔断
  - API 聚合

**计划中**:
- 📋 **tenant-service** - 完整租户管理
  - 租户配额管理
  - 租户计费集成

### 核心特性
- 多租户架构 (Pool/Bridge/Silo 隔离策略)
- JWT 认证 + 基于角色的访问控制 (RBAC)
- 事件驱动架构 (NATS JetStream)
- 完整的可观测性 (OpenTelemetry)

## 项目结构

```
/home/xl/play/saas/
├── crates/                    # Rust 微服务
│   ├── api-gateway/           # API 网关 (Axum)
│   ├── auth-service/          # 认证服务
│   ├── tenant-service/        # 租户管理服务
│   └── shared/                # 共享库 (工具、中间件、类型)
├── deploy/k8s/                # Kubernetes 部署清单
├── infrastructure/
│   ├── local/                 # Docker Compose (本地开发)
│   └── terraform/             # AWS 基础设施 (参考用)
├── migrations/                # SQLx 数据库迁移
├── docs/                      # 架构文档
├── scripts/                   # 工具脚本
├── Makefile                   # 构建命令
└── Cargo.toml                 # Rust workspace 配置
```

## 技术栈

### 后端框架
- **Axum 0.8** - Web 框架
- **Tokio** - 异步运行时
- **Tower** - 中间件生态

### 数据层
- **SQLx 0.8** - 数据库驱动 (PostgreSQL)
- **Redis** - 缓存 + 会话存储
- **NATS JetStream** - 消息队列

### 认证与安全
- **jsonwebtoken** - JWT 令牌
- **argon2** - 密码哈希
- **RSA** - 非对称加密

### 可观测性
- **tracing** - 日志
- **OpenTelemetry** - 链路追踪 + 指标
- **Prometheus** - 指标收集

## 编码规范

### Rust 代码风格
```rust
// 使用 thiserror 定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Tenant not found: {0}")]
    TenantNotFound(uuid::Uuid),
}

// 使用 anyhow 处理应用错误
pub type Result<T> = std::result::Result<T, Error>;
```

### 模块组织
```rust
// 每个 service crate 标准结构
src/
├── main.rs           // 入口 + 路由配置
├── config.rs         // 配置管理
├── handlers.rs       // HTTP 处理器
├── models.rs         // 数据模型
├── repository.rs     // 数据库操作
├── service.rs        // 业务逻辑
└── telemetry.rs      // 可观测性
```

### 中间件使用
```rust
// 使用 shared crate 的中间件
use shared::middleware::{auth, tenant, rate_limit};

let app = Router::new()
    .route("/api/v1/users", get(list_users))
    .layer(middleware::from_fn(auth::require_auth))
    .layer(middleware::from_fn(tenant::extract_tenant));
```

### 环境变量命名
```bash
# 使用双下划线分隔层级
DATABASE__URL=postgresql://...
SERVER__HOST=0.0.0.0
JWT__ISSUER=erp-saas
```

## 租户隔离策略

| 级别 | 模式 | 数据库 | Schema | 适用场景 |
|------|------|--------|--------|----------|
| **Pool** | 共享数据库 | 1 | 1 | Starter 套餐 |
| **Bridge** | 独立 Schema | 1 | N | Pro 套餐 |
| **Silo** | 独立数据库 | N | N | Enterprise 套餐 |

### 租户上下文
```rust
// 从请求中提取租户信息
use shared::tenant::TenantContext;

async fn handler(
    Extension(tenant): Extension<TenantContext>,
) -> impl IntoResponse {
    // tenant.id - 租户 UUID
    // tenant.isolation_level - 隔离级别
    // tenant.schema - Schema 名称 (Bridge 模式)
}
```

## 开发命令

```bash
# 本地开发
just dev-up              # 启动基础设施 (Docker Compose)
just dev-down            # 停止基础设施
just dev                 # 启动开发服务器
just test                # 运行测试
just test-coverage      # 生成测试覆盖率报告
just lint                # 代码检查 (cargo clippy)

# 数据库
just db-migrate          # 运行迁移
just db-rollback         # 回滚迁移
just db-reset            # 重置数据库

# Docker
just docker-build        # 构建镜像

# Kubernetes (可选)
just k3s-setup           # 安装 K3s
just k3s-deploy          # 部署到 K3s
```

### Git Hooks (prek)

项目配置了 prek hooks 用于自动代码质量检查。**在首次修改文件前**应确保 hooks 已安装。

**安装**:
```bash
# 安装 cargo-binstall (如果尚未安装)
cargo install cargo-binstall

# 使用 binstall 快速安装 prek
cargo binstall prek
prek install
```

**Hooks 包括**:
- `cargo fmt` - 代码格式化
- `cargo clippy` - Lint 检查
- `cargo test` - 单元测试（本地开发时手动运行）
- 文件大小检查
- 拼写错误检查
- 安全扫描 (gitleaks)

> **注意**: 首次修改文件前请运行 `prek install`，否则提交会失败。

## 关键技术决策

### 1. 为什么选择 Rust?
- 高性能: 零成本抽象，无 GC 暂停
- 安全性: 编译时内存安全保证
- 并发: 所有权系统实现无数据竞争并发
- 生态: Axum/SQLx/Tokio 成熟稳定

### 2. 为什么选择 SQLx 而不是 Diesel?
- 异步原生支持
- 编译时 SQL 检查 (不依赖宏)
- 更轻量，更灵活
- 原生支持 PostgreSQL 高级特性

### 3. 为什么选择 NATS 而不是 Kafka?
- 更轻量，适合中小规模
- 原生支持 JetStream (持久化)
- 更简单的运维
- 支持 Request-Reply 模式

### 4. 多租户架构选择
- **Pool**: 成本最低，适合中小企业
- **Bridge**: 平衡成本与隔离，适合中型企业
- **Silo**: 最高隔离级别，适合大型企业/合规要求

## 本地开发环境

### 微服务端口配置
| 服务 | 默认端口 | 说明 |
|------|---------|------|
| api-gateway | 8080 | API 网关入口 |
| auth-service | 8081 | 认证服务 |

### 环境变量配置
```bash
# API Gateway 配置
SERVER__PORT=8080

# 认证服务 URL (api-gateway 调用 auth-service)
AUTH_SERVICE__URL=http://127.0.0.1:8081

# Auth Service 配置 (如需修改端口)
# SERVER__PORT=8081
```

### 基础设施 (Docker Compose)
| 服务 | 端口 | 用途 |
|------|------|------|
| PostgreSQL | 5432 | 主数据库 |
| Redis | 6379 | 缓存 + 会话 |
| NATS | 4222 | 消息队列 |
| MinIO | 9000/9001 | S3 兼容存储 |
| LocalStack | 4566 | AWS 服务模拟 |
| MailHog | 1025/8025 | 邮件测试 |

### 初始化脚本
```bash
./scripts/setup.sh
```
会自动:
1. 检查依赖 (Rust, Docker, cargo-watch)
2. 生成 JWT 密钥 (keys/)
3. 创建 .env 文件
4. 启动 Docker 服务
5. 运行数据库迁移

## 安全注意事项

### 敏感信息处理
- **永远不要**提交真实凭证到 git
- 使用 `.env.example` 作为模板
- 生产环境使用 Kubernetes Secret 或 AWS Secrets Manager
- JWT 密钥使用 RSA 2048 位以上

### CORS 配置
```rust
// 不要使用 allow_origin(Any)
let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::exact(
        http::HeaderValue::from_static("http://localhost:3000")
    ));
```

### SQL 注入防护
```rust
// 使用参数化查询
sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE tenant_id = $1 AND id = $2",
    tenant_id,
    user_id
)
.fetch_one(&pool)
.await?;
```

## 常见任务

### 添加新的 API 端点
1. 在 `handlers.rs` 添加处理函数
2. 在 `models.rs` 添加请求/响应模型
3. 在 `main.rs` 注册路由
4. 在 `repository.rs` 添加数据访问方法

### 添加新的微服务
1. 创建 `crates/new-service/`
2. 添加 `Cargo.toml` (依赖 shared)
3. 在 workspace `Cargo.toml` 添加成员
4. 更新 `docker-compose.yaml`
5. 添加 Kubernetes 配置

### 数据库迁移
```bash
# 1) 在 migrations/ 下创建新文件
#    <timestamp>_<action>_<object>.sql

# 2) 更新 migrations/LATEST_VERSION 为最新版本号

# 3) 运行迁移
just db-migrate
```

详细规范见 `docs/database-migrations.md`（命名规范、幂等要求、回滚策略、PR Checklist）。

## 测试策略

### 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_password_hashing() {
        let password = "test_password";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash).unwrap());
    }
}
```

### 集成测试
```rust
// 使用 sqlx 的测试池
#[sqlx::test]
async fn test_user_creation(pool: PgPool) {
    let repo = UserRepository::new(pool);
    // ... 测试逻辑
}
```

## 性能优化建议

1. **数据库连接池**: 设置合适的 `min_connections` 和 `max_connections`
2. **Redis 缓存**: 缓存频繁访问的数据 (租户配置、权限列表)
3. **异步处理**: 使用 NATS 处理耗时任务 (邮件发送、报表生成)
4. **查询优化**: 使用 `sqlx::query_as!` 进行编译时检查

## 文档资源

- [架构设计](docs/architecture.md) - 系统架构详解
- [迁移开发指南](docs/database-migrations.md) - 数据库迁移编写与验证规范
- [租户隔离](docs/tenant-isolation.md) - 多租户隔离策略
- [安全设计](docs/security.md) - 安全措施与最佳实践
- [部署指南](docs/deployment-guide.md) - 部署流程与配置

## AI 助手注意事项

1. **代码生成**: 遵循现有的模块结构和命名规范
2. **错误处理**: 使用 `thiserror` 定义错误，`anyhow` 处理应用错误
3. **异步代码**: 所有 I/O 操作使用 async/await
4. **类型安全**: 充分利用 Rust 的类型系统，避免使用 `unwrap()`
5. **测试覆盖**: 为新功能编写单元测试和集成测试
6. **文档注释**: 为公共 API 添加文档注释 (`///`)
7. **安全性**: 永远不要生成包含硬编码凭证的代码
8. **租户感知**: 所有数据访问必须考虑租户隔离

---

**最后更新**: 2026-03-10
**维护者**: ERP SaaS Team
