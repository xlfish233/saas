#!/bin/bash
# 开发环境初始化脚本
# 用法: ./scripts/setup.sh

set -e

echo "================================================"
echo "  ERP SaaS - 开发环境初始化"
echo "================================================"

# 检查 Rust
echo "🔍 检查 Rust..."
if ! command -v rustc &> /dev/null; then
    echo "📦 安装 Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi
echo "   Rust: $(rustc --version)"

# 检查 Cargo 工具
echo "🔍 检查 Cargo 工具..."

if ! command -v sqlx &> /dev/null; then
    echo "📦 安装 sqlx-cli..."
    cargo install sqlx-cli --no-default-features --features native-tls,postgres
fi

if ! command -v cargo-watch &> /dev/null; then
    echo "📦 安装 cargo-watch..."
    cargo install cargo-watch
fi

# 检查 Docker
echo "🔍 检查 Docker..."
if ! command -v docker &> /dev/null; then
    echo "❌ Docker 未安装，请先安装 Docker"
    exit 1
fi
echo "   Docker: $(docker --version)"

# 检查 Docker Compose
if ! command -v docker &> /dev/null; then
    echo "❌ Docker Compose 未安装，请先安装"
    exit 1
fi
echo "   Docker Compose: $(docker compose version)"

# 生成 JWT 密钥
echo "🔐 生成 JWT 密钥..."
mkdir -p keys
if [ ! -f keys/private.pem ]; then
    openssl genrsa -out keys/private.pem 2048
    openssl rsa -in keys/private.pem -pubout -out keys/public.pem
    echo "   ✅ 密钥已生成"
else
    echo "   ⏭️  密钥已存在"
fi

# 创建环境变量文件
echo "📝 创建环境变量文件..."
if [ ! -f .env ]; then
    cat > .env <<EOF
# 数据库
DATABASE_URL=postgresql://dev_user:dev_password@localhost:5432/dev_db

# Redis
REDIS_URL=redis://localhost:6379

# NATS
NATS_URL=nats://localhost:4222

# S3 / MinIO
S3_ENDPOINT=http://localhost:9000
AWS_ACCESS_KEY_ID=local_dev_user
AWS_SECRET_ACCESS_KEY=local_dev_secret
AWS_REGION=us-east-1

# JWT
JWT_PRIVATE_KEY_PATH=./keys/private.pem
JWT_PUBLIC_KEY_PATH=./keys/public.pem
JWT_ISSUER=erp-saas
JWT_AUDIENCE=erp-saas-api

# 服务
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# 环境
ENVIRONMENT=local
RUST_LOG=debug,sqlx=info
EOF
    echo "   ✅ .env 已创建"
else
    echo "   ⏭️  .env 已存在"
fi

# 启动基础设施
echo ""
echo "================================================"
echo "  🚀 启动本地基础设施..."
echo "================================================"
docker compose -f infrastructure/local/docker-compose.yaml up -d

echo "⏳ 等待服务就绪..."
sleep 5

# 检查服务健康
echo "🔍 检查服务状态..."
docker compose -f infrastructure/local/docker-compose.yaml ps

# 运行迁移
echo ""
echo "================================================"
echo "  📦 运行数据库迁移..."
echo "================================================"
cargo sqlx migrate run

# 完成
echo ""
echo "================================================"
echo "  ✅ 开发环境初始化完成!"
echo "================================================"
echo ""
echo "服务端点:"
echo "  API:          http://localhost:8080"
echo "  PostgreSQL:   localhost:5432"
echo "  Redis:        localhost:6379"
echo "  NATS:         localhost:4222"
echo "  MinIO API:    http://localhost:9000"
echo "  MinIO Console: http://localhost:9001"
echo ""
echo "下一步:"
echo "  make dev      # 启动开发服务器"
echo "  make test     # 运行测试"
echo ""
