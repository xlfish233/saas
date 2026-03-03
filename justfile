# ERP SaaS Platform - 命令运行器
# 使用 `just --list` 查看所有可用命令

# 配置变量
image_name := "erp-saas"
image_tag := "latest"
registry := `aws ecr describe-repositories --repository-name erp-saas --query 'repositories[0].repositoryUri' --output text 2>/dev/null || echo "local"`

# 默认命令：显示帮助
default:
    just --list

# ============================================
# 本地开发
# ============================================

# 启动本地基础设施 (Docker Compose)
dev-up:
    echo "🚀 启动本地基础设施..."
    docker compose -f infrastructure/local/docker-compose.yaml up -d
    echo "⏳ 等待服务就绪..."
    sleep 5
    echo "✅ 服务已启动:"
    echo "   PostgreSQL: localhost:5432"
    echo "   Redis:      localhost:6379"
    echo "   NATS:       localhost:4222"
    echo "   MinIO:      localhost:9000 (Console: 9001)"

# 停止本地基础设施
dev-down:
    echo "🛑 停止本地基础设施..."
    docker compose -f infrastructure/local/docker-compose.yaml down

# 启动开发服务器
dev:
    echo "🔧 启动开发服务器..."
    cargo run --bin api-gateway

# 运行测试
test:
    echo "🧪 运行测试..."
    cargo test --all-features --no-fail-fast

# 生成测试覆盖率报告
test-coverage:
    echo "📊 生成测试覆盖率报告..."
    cargo tarpaulin --out Xml --output-file cobertura.xml --out Html --output-file tarpaulin-report.html --skip-clean --engine llvm

# 查看覆盖率报告
test-coverage-view:
    echo "👀 打开覆盖率报告..."
    if [ ! -f tarpaulin-report.html ]; then \
        echo "Error: tarpaulin-report.html not found. Run 'just test-coverage' first."; \
        exit 1; \
    fi
    open tarpaulin-report.html

# 代码检查 (clippy + fmt)
lint:
    echo "🔍 代码检查..."
    cargo clippy --all-targets -- -D warnings
    cargo fmt --check

# 依赖安全审计
audit:
    echo "🔐 运行依赖安全审计..."
    cargo audit --deny warnings

# ============================================
# 数据库迁移
# ============================================

# 运行数据库迁移
db-migrate:
    echo "📦 运行数据库迁移..."
    sqlx migrate run

# 回滚迁移
db-rollback:
    echo "↩️ 回滚迁移..."
    sqlx migrate revert

# 重置数据库
db-reset:
    echo "🔄 重置数据库..."
    sqlx database drop -y
    sqlx database create
    sqlx migrate run

# ============================================
# Docker
# ============================================

# 构建 Docker 镜像
docker-build:
    echo "🏗️ 构建 Docker 镜像..."
    docker build -t {{image_name}}:{{image_tag}} .

# 推送镜像到 ECR
docker-push:
    echo "📤 推送镜像到 ECR..."
    aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin {{registry}}
    docker tag {{image_name}}:{{image_tag}} {{registry}}:{{image_tag}}
    docker push {{registry}}:{{image_tag}}

# ============================================
# K3s (本地 Kubernetes)
# ============================================

# 安装 K3s
k3s-setup:
    echo "⚙️ 安装 K3s..."
    ./scripts/k3s-setup.sh

# 部署到 K3s
k3s-deploy:
    echo "🚀 部署到 K3s..."
    kubectl apply -k deploy/k8s/overlays/local

# 删除 K3s 部署
k3s-down:
    echo "🗑️ 删除 K3s 部署..."
    kubectl delete -k deploy/k8s/overlays/local

# 查看 K3s 日志
k3s-logs:
    kubectl logs -l app=api-gateway -f --all-containers

# ============================================
# AWS EKS
# ============================================

# 初始化 Terraform
tf-init:
    echo "🔧 初始化 Terraform..."
    cd infrastructure/terraform && terraform init

# Terraform 计划
tf-plan:
    echo "📋 Terraform 计划..."
    cd infrastructure/terraform && terraform plan

# 应用 Terraform
tf-apply:
    echo "🚀 应用 Terraform..."
    cd infrastructure/terraform && terraform apply -auto-approve

# 销毁基础设施
tf-destroy:
    echo "⚠️ 销毁基础设施..."
    cd infrastructure/terraform && terraform destroy

# 部署到 EKS
eks-deploy:
    echo "🚀 部署到 EKS..."
    kubectl apply -k deploy/k8s/overlays/production

# ============================================
# 监控
# ============================================

# 部署监控栈
monitoring-up:
    echo "📊 部署监控栈..."
    kubectl apply -f deploy/k8s/monitoring/

# 端口转发 Grafana
port-forward-grafana:
    kubectl port-forward -n monitoring svc/grafana 3000:80

# 端口转发 Prometheus
port-forward-prometheus:
    kubectl port-forward -n monitoring svc/prometheus 9090:9090
