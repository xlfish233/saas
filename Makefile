.PHONY: help dev dev-up dev-down test lint db-migrate db-rollback
.PHONY: docker-build docker-push k3s-setup k3s-deploy k3s-down
.PHONY: tf-init tf-plan tf-apply tf-destroy eks-deploy

# 配置
IMAGE_NAME ?= erp-saas
IMAGE_TAG ?= latest
REGISTRY ?= $(shell aws ecr describe-repositories --repository-name erp-saas --query 'repositories[0].repositoryUri' --output text 2>/dev/null || echo "local")

# 默认目标
help:
	@echo "ERP SaaS Platform - 可用命令:"
	@echo ""
	@echo "本地开发:"
	@echo "  make dev-up        启动本地基础设施 (Docker Compose)"
	@echo "  make dev-down      停止本地基础设施"
	@echo "  make dev           启动开发服务器"
	@echo "  make test          运行测试"
	@echo "  make lint          代码检查 (clippy + fmt)"
	@echo ""
	@echo "数据库:"
	@echo "  make db-migrate   运行迁移"
	@echo "  make db-rollback  回滚迁移"
	@echo "  make db-reset     重置数据库"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-build 构建镜像"
	@echo "  make docker-push  推送镜像"
	@echo ""
	@echo "K3s (本地 K8s):"
	@echo "  make k3s-setup    安装 K3s"
	@echo "  make k3s-deploy   部署到 K3s"
	@echo "  make k3s-down     删除 K3s 部署"
	@echo ""
	@echo "AWS EKS:"
	@echo "  make tf-init      初始化 Terraform"
	@echo "  make tf-plan      Terraform 计划"
	@echo "  make tf-apply     应用 Terraform"
	@echo "  make tf-destroy   销毁基础设施"
	@echo "  make eks-deploy   部署到 EKS"

# ============================================
# 本地开发
# ============================================

dev-up:
	@echo "🚀 启动本地基础设施..."
	docker compose -f infrastructure/local/docker-compose.yaml up -d
	@echo "⏳ 等待服务就绪..."
	@sleep 5
	@echo "✅ 服务已启动:"
	@echo "   PostgreSQL: localhost:5432"
	@echo "   Redis:      localhost:6379"
	@echo "   NATS:       localhost:4222"
	@echo "   MinIO:      localhost:9000 (Console: 9001)"

dev-down:
	@echo "🛑 停止本地基础设施..."
	docker compose -f infrastructure/local/docker-compose.yaml down

dev:
	@echo "🔧 启动开发服务器..."
	cargo run --bin api-gateway

test:
	@echo "🧪 运行测试..."
	cargo test --all-features --no-fail-fast

test-coverage:
	@echo "📊 生成测试覆盖率报告..."
	cargo tarpaulin --out Xml --output-file cobertura.xml --out Html --output-file tarpaulin-report.html --skip-clean --engine llvm

test-coverage-view:
	@echo "👀 打开覆盖率报告..."
	@if [ ! -f tarpaulin-report.html ]; then \
		echo "Error: tarpaulin-report.html not found. Run 'make test-coverage' first."; \
		exit 1; \
	@ ; fi
	open tarpaulin-report.html

lint:
	@echo "🔍 代码检查..."
	cargo clippy --all-targets -- -D warnings
	cargo fmt --check

# ============================================
# 数据库迁移
# ============================================

db-migrate:
	@echo "📦 运行数据库迁移..."
	sqlx migrate run

db-rollback:
	@echo "↩️ 回滚迁移..."
	sqlx migrate revert

db-reset:
	@echo "🔄 重置数据库..."
	sqlx database drop -y
	sqlx database create
	sqlx migrate run

# ============================================
# Docker
# ============================================

docker-build:
	@echo "🏗️ 构建 Docker 镜像..."
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .

docker-push:
	@echo "📤 推送镜像到 ECR..."
	aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin $(REGISTRY)
	docker tag $(IMAGE_NAME):$(IMAGE_TAG) $(REGISTRY):$(IMAGE_TAG)
	docker push $(REGISTRY):$(IMAGE_TAG)

# ============================================
# K3s (本地 Kubernetes)
# ============================================

k3s-setup:
	@echo "⚙️ 安装 K3s..."
	./scripts/k3s-setup.sh

k3s-deploy:
	@echo "🚀 部署到 K3s..."
	kubectl apply -k deploy/k8s/overlays/local

k3s-down:
	@echo "🗑️ 删除 K3s 部署..."
	kubectl delete -k deploy/k8s/overlays/local

k3s-logs:
	kubectl logs -l app=api-gateway -f --all-containers

# ============================================
# AWS EKS
# ============================================

tf-init:
	@echo "🔧 初始化 Terraform..."
	cd infrastructure/terraform && terraform init

tf-plan:
	@echo "📋 Terraform 计划..."
	cd infrastructure/terraform && terraform plan

tf-apply:
	@echo "🚀 应用 Terraform..."
	cd infrastructure/terraform && terraform apply -auto-approve

tf-destroy:
	@echo "⚠️ 销毁基础设施..."
	cd infrastructure/terraform && terraform destroy

eks-deploy:
	@echo "🚀 部署到 EKS..."
	kubectl apply -k deploy/k8s/overlays/production

# ============================================
# 监控
# ============================================

monitoring-up:
	@echo "📊 部署监控栈..."
	kubectl apply -f deploy/k8s/monitoring/

port-forward-grafana:
	kubectl port-forward -n monitoring svc/grafana 3000:80

port-forward-prometheus:
	kubectl port-forward -n monitoring svc/prometheus 9090:9090
