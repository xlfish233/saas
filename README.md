# ERP SaaS Platform

云原生多租户 ERP SaaS 平台，基于 Rust 构建的后端服务。

> ⚠️ **当前阶段**: 本地开发优先，暂不部署到 AWS

## 快速开始

### 前置条件

- Docker & Docker Compose
- Rust 1.75+
- sqlx-cli (`cargo install sqlx-cli`)
- Make (可选)
- prek (可选, `cargo binstall prek`)

### 1. 本地开发 (推荐)

```bash
# 安装依赖并生成配置
./scripts/setup.sh

# 可选：安装 prek git hooks
prek install

# 启动基础设施 (PostgreSQL, Redis, NATS, MinIO, LocalStack)
make dev-up

# 等待服务启动 (约 10 秒)
sleep 10

# 运行数据库迁移
make db-migrate

# 启动 API 服务
make dev

# 测试
curl http://localhost:8080/health
```

### 2. K3s 本地 Kubernetes (可选)

```bash
# 安装 k3s 并部署
make k3s-setup
make k3s-deploy
```

### ~~3. AWS EKS 生产部署~~

> ⚠️ **暂不支持** - 当前专注于本地开发环境，AWS 部署配置仅供参考

## Mock 服务说明

本地开发使用以下 Mock 服务：

| 服务 | 用途 | 端口 |
|-----|------|------|
| PostgreSQL | 数据库 | 5432 |
| Redis | 缓存 | 6379 |
| NATS | 消息队列 | 4222 |
| MinIO | S3 兼容存储 | 9000 (API), 9001 (Console) |
| LocalStack | AWS 服务模拟 | 4566 |
| MailHog | 邮件测试 | 1025 (SMTP), 8025 (Web) |

**默认开发凭证** (仅用于本地开发):
- PostgreSQL: `erp:erp123`
- MinIO: `minioadmin:minioadmin`

> ⚠️ 这些凭证仅用于本地开发，生产环境必须使用强密码和密钥管理服务

## 项目结构

```
├── crates/                 # Rust 服务
│   ├── api-gateway/        # API 网关
│   ├── auth-service/       # 认证服务
│   ├── tenant-service/     # 租户管理
│   └── shared/             # 共享库
├── deploy/                 # 部署配置
│   ├── k8s/                # Kubernetes 清单
│   └── helm/               # Helm Charts
├── infrastructure/         # 基础设施
│   ├── terraform/          # AWS 资源
│   └── local/              # 本地开发
├── migrations/             # 数据库迁移
├── docs/                   # 文档
│   ├── architecture.md     # 架构设计
│   ├── tenant-isolation.md # 租户隔离
│   └── security.md         # 安全设计
└── scripts/                # 工具脚本
```

## 部署层级

| 层级 | 环境 | 用途 | 文档 |
|-----|-----|-----|-----|
| L0 | Docker Compose | 快速验证 | [docs/local-dev.md](docs/local-dev.md) |
| L1 | K3s | 本地 K8s | [docs/k3s.md](docs/k3s.md) |
| L2 | AWS EKS | 生产 | [docs/aws-eks.md](docs/aws-eks.md) |

## 租户隔离策略

| 级别 | 模式 | 适用场景 | 隔离程度 |
|-----|-----|---------|---------|
| Pool | 共享数据库 | Starter | 逻辑隔离 |
| Bridge | 独立 Schema | Pro | Schema 隔离 |
| Silo | 独立数据库 | Enterprise | 物理隔离 |

详见 [docs/tenant-isolation.md](docs/tenant-isolation.md)

## 技术栈

- **后端**: Rust (Axum, SQLx, Tokio)
- **数据库**: Aurora PostgreSQL + RDS Proxy
- **缓存**: ElastiCache Redis
- **消息队列**: NATS JetStream
- **对象存储**: S3
- **容器编排**: Kubernetes (EKS)
- **服务网格**: Istio (可选)
- **监控**: Prometheus + Grafana + OpenTelemetry

## 开发工具

### Git Hooks (prek)

项目使用 [prek](https://prek.j178.dev/) - 一个用 Rust 重写的高性能 pre-commit 框架，速度比 Python 版本快 5-6 倍。

**安装**:
```bash
# 安装 cargo-binstall (如果尚未安装)
cargo install cargo-binstall

# 使用 binstall 快速安装 prek (从预编译二进制)
cargo binstall prek

# 安装 git hooks
prek install

# (可选) 手动运行所有 hooks
prek run --all-files
```

**自动运行的检查**:
- `cargo fmt` - 代码格式化
- `cargo clippy` - Linting 检查
- `cargo test` - 单元测试（本地）
- 文件大小检查
- 拼写错误检查
- 行尾空白检查
- 安全扫描 (gitleaks)

> **优势**: prek 完全兼容现有的 `.pre-commit-config.yaml` 配置，无需修改任何配置文件即可享受 Rust 带来的性能提升。

## 常用命令

```bash
make help                 # 显示所有命令
make dev-up               # 启动本地基础设施
make dev-down             # 停止本地基础设施
make test                 # 运行测试
make lint                 # 代码检查
make db-migrate           # 数据库迁移
make db-rollback          # 回滚迁移
make docker-build         # 构建镜像
make k3s-deploy           # 部署到 K3s
make tf-plan              # Terraform 计划
make tf-apply             # Terraform 应用
```

## License

MIT
