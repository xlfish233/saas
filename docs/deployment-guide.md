# 部署指南

## 部署层级概览

```
┌─────────────────────────────────────────────────────────────────┐
│                      部署层级                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  L0: Docker Compose    L1: K3s          L2: AWS EKS            │
│  ┌──────────────┐     ┌──────────────┐  ┌──────────────────────┐│
│  │  本地开发     │     │  本地 K8s    │  │  生产集群            ││
│  │              │     │              │  │                      ││
│  │ - PostgreSQL │     │ - 单节点     │  │ - 多节点 HA          ││
│  │ - Redis      │     │ - SQLite     │  │ - Aurora PostgreSQL ││
│  │ - NATS       │     │ - 本地存储   │  │ - ElastiCache       ││
│  │ - MinIO      │     │              │  │ - NATS (HA)         ││
│  │              │     │              │  │ - S3                ││
│  │ 最快验证     │     │ K8s 体验     │  │ 生产就绪            ││
│  └──────────────┘     └──────────────┘  └──────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## L0: 本地开发 (Docker Compose)

### 快速启动

```bash
# 1. 启动基础设施
just dev-up

# 2. 运行迁移
just db-migrate

# 3. 启动服务
just dev
```

### 服务端点

| 服务 | 端口 | 用途 |
|-----|-----|-----|
| API | 8080 | 主服务 |
| PostgreSQL | 5432 | 数据库 |
| Redis | 6379 | 缓存 |
| NATS | 4222 | 消息队列 |
| MinIO API | 9000 | 对象存储 |
| MinIO Console | 9001 | 管理界面 |
| LocalStack | 4566 | AWS 模拟 |
| MailHog SMTP | 1025 | 邮件发送 |
| MailHog Web | 8025 | 邮件查看 |

### 环境变量

```bash
# .env.local
DATABASE__URL=postgresql://placeholder_user:placeholder_password@localhost:5432/placeholder_db
REDIS__URL=redis://localhost:6379
NATS__URL=nats://localhost:4222
S3_ENDPOINT=http://localhost:9000
AWS_ACCESS_KEY_ID=local_dev_user
AWS_SECRET_ACCESS_KEY=local_dev_secret
JWT__PRIVATE_KEY_PATH=./keys/private.pem
JWT__PUBLIC_KEY_PATH=./keys/public.pem
SERVER__ENVIRONMENT=local
RUST_LOG=debug
```

## L1: K3s 本地 Kubernetes

### 安装 K3s

```bash
# 使用脚本安装
./scripts/k3s-setup.sh

# 或手动安装
curl -sfL https://get.k3s.io | sh -s - --disable traefik
```

### 部署应用

```bash
# 部署到 K3s
just k3s-deploy

# 查看状态
kubectl get pods -A

# 查看日志
just k3s-logs

# 端口转发
kubectl port-forward svc/api-gateway 8080:80
```

### K3s 配置

```yaml
# deploy/k8s/overlays/local/kustomization.yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - ../../base

patches:
  - target:
      kind: Deployment
      name: api-gateway
    patch: |-
      - op: replace
        path: /spec/replicas
        value: 1
      - op: replace
        path: /spec/template/spec/containers/0/resources
        value:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
```

## L2: AWS EKS 生产部署

### 前置条件

- AWS CLI 配置完成
- kubectl 安装
- Terraform 1.5+

### 基础设施部署

```bash
# 1. 初始化 Terraform
just tf-init

# 2. 查看计划
just tf-plan

# 3. 应用基础设施
just tf-apply    # 约 20-30 分钟
```

### 获取集群凭证

```bash
aws eks update-kubeconfig \
  --region us-east-1 \
  --name erp-saas-cluster
```

### 部署应用

```bash
# 构建并推送镜像
just docker-build
just docker-push

# 部署到 EKS
just eks-deploy
```

### 生产配置

```yaml
# deploy/k8s/overlays/production/kustomization.yaml
apiVersion: kustomize.config.k8s.io/v1beta1
kind: Kustomization

resources:
  - ../../base

patches:
  - target:
      kind: Deployment
    patch: |-
      - op: replace
        path: /spec/replicas
        value: 3
      - op: add
        path: /spec/template/spec/containers/0/resources
        value:
          requests:
            cpu: 250m
            memory: 512Mi
          limits:
            cpu: 1000m
            memory: 1Gi

images:
  - name: erp-saas
    newName: ${ECR_REGISTRY}/erp-saas
    newTag: ${IMAGE_TAG}
```

## 监控部署

```bash
# 部署监控栈
just monitoring-up

# 访问 Grafana
just port-forward-grafana
# http://localhost:3000 (admin/admin)

# 访问 Prometheus
just port-forward-prometheus
# http://localhost:9090
```

## 回滚

```bash
# K3s 回滚
kubectl rollout undo deployment/api-gateway

# EKS 回滚
kubectl rollout undo deployment/api-gateway -n production

# 查看历史
kubectl rollout history deployment/api-gateway
```

## 常见问题

### 1. 数据库连接失败

```bash
# 检查服务状态
docker compose ps postgres

# 查看日志
docker compose logs postgres
```

### 2. K3s Pod 无法启动

```bash
# 查看事件
kubectl describe pod <pod-name>

# 查看日志
kubectl logs <pod-name> --previous
```

### 3. EKS 节点 Not Ready

```bash
# 检查节点状态
kubectl describe node <node-name>

# 检查 AWS 资源
aws ec2 describe-instances --filters "Name=tag:Name,Values=*erp-saas*"
```
