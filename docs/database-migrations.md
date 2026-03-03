# 数据库迁移开发指南

本文档定义 ERP SaaS 项目的数据库迁移编写规范，目标是让迁移具备可重复、可回滚、可审计的工程质量。

## 当前迁移模型

- 迁移目录: `migrations/`
- 版本基线: `migrations/LATEST_VERSION`
- 迁移执行:
  - `api-gateway` 启动时以 owner 角色自动执行 `migrate up`
  - `auth-service` / `tenant-service` 启动时仅做版本门禁（verifier）
- 角色约束: 三个服务在代码中固定角色，避免通过环境变量误配造成并发迁移
- 手动入口: `just db-migrate`（底层为 `db-migrator`）

### 独立迁移作业最小配置

`db-migrator` 只依赖数据库配置，不要求 `SERVER__*`、`REDIS__*`、`NATS__*`、`JWT__*`。

最小必需环境变量:

- `DATABASE__URL`

可选环境变量:

- `DATABASE__POOL_SIZE`
- `DATABASE__MIGRATION__ENABLED`
- `DATABASE__MIGRATION__MAX_RETRIES`
- `DATABASE__MIGRATION__BASE_DELAY_MS`
- `DATABASE__MIGRATION__REQUIRED_VERSION`

## 迁移文件命名

使用格式:

```text
<timestamp>_<action>_<object>.sql
```

示例:

- `20260303120000_add_index_users_email.sql`
- `20260303121000_add_column_tenants_timezone.sql`

要求:

- `timestamp` 使用 UTC 时间，精确到秒，且严格递增。
- 文件名必须能表达“做了什么”。

## SQL 编写规范

### 1) 幂等优先

正确示例:

```sql
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
ALTER TABLE users ADD COLUMN IF NOT EXISTS timezone TEXT;
```

错误示例:

```sql
CREATE INDEX idx_users_email ON users(email);
ALTER TABLE users ADD COLUMN timezone TEXT;
```

### 2) 避免一次迁移里混入多类高风险变更

建议:

- DDL（结构）与大批量 DML（数据回填）拆分为多个迁移。
- 索引创建、列变更、数据修复分开提交，便于定位与回滚。

### 3) 大表变更采用分阶段

推荐顺序:

1. 新增可空列
2. 业务代码双写/回填
3. 校验完成后再加 `NOT NULL` / 删除旧列

## 注释与可读性

每个迁移文件顶部必须包含:

```sql
-- Purpose: 为 users.email 添加查询索引
-- Risk: 低（仅索引创建）
-- Rollback: DROP INDEX IF EXISTS idx_users_email;
```

## 版本基线规则（必须）

每新增一个迁移文件，必须同步更新 `migrations/LATEST_VERSION` 为最新版本号。

示例:

```text
20260303120000
```

如果 `LATEST_VERSION` 未更新，版本门禁会导致 verifier 服务拒绝启动。

## 本地开发流程

1. 新建迁移文件到 `migrations/`
2. 更新 `migrations/LATEST_VERSION`
3. 执行迁移:

```bash
just db-migrate
```

4. 查看版本:

```bash
cargo run --bin db-migrator -- version
```

5. 验证门禁:

```bash
cargo run --bin db-migrator -- verify
```

6. 回滚最近一次迁移（需要存在对应 `.down.sql`）:

```bash
just db-rollback
```

7. 重置数据库并重跑迁移:

```bash
just db-reset
```

## 故障处理 SOP

1. 迁移失败时不要强行启动服务。
2. 修复迁移 SQL 后重新执行 `just db-migrate`。
3. 如已产生部分副作用，先按迁移文件里的 rollback 说明清理，再重试。
4. 不可逆变更必须先做备份并在 PR 中标注恢复方案。

## PR Checklist（迁移相关）

提交包含迁移时，PR 描述必须包含以下清单:

- [ ] 迁移文件名符合规范，版本号递增
- [ ] 已更新 `migrations/LATEST_VERSION`
- [ ] SQL 具备幂等性（`IF EXISTS / IF NOT EXISTS`）
- [ ] 已描述回滚方案（可逆 SQL 或补偿策略）
- [ ] 已在本地执行 `just db-migrate` 并通过 `db-migrator verify`
- [ ] 已评估锁表/性能影响（尤其是索引和大表变更）

## 当前范围声明

当前自动化迁移仅覆盖 Pool 主库。Bridge/Silo 的 schema/db 生命周期迁移将在后续版本扩展。
