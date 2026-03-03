#!/bin/bash
# K3s 安装脚本
# 用法: ./scripts/k3s-setup.sh

set -e

echo "================================================"
echo "  ERP SaaS - K3s 本地 Kubernetes 安装"
echo "================================================"

# 检查是否已安装
if command -v k3s &> /dev/null; then
    echo "⚠️  K3s 已安装"
    k3s --version
    exit 0
fi

# 检查系统要求
echo "🔍 检查系统要求..."
if [ "$(id -u)" -ne 0 ]; then
    echo "❌ 请使用 root 权限运行此脚本"
    echo "   sudo ./scripts/k3s-setup.sh"
    exit 1
fi

# 安装 K3s
echo "📦 安装 K3s (禁用 Traefik)..."
curl -sfL https://get.k3s.io | sh -s - \
    --disable traefik \
    --write-kubeconfig-mode 644 \
    --kubelet-arg="--max-pods=100"

# 等待 K3s 就绪
echo "⏳ 等待 K3s 就绪..."
sleep 10

# 检查节点状态
echo "🔍 检查节点状态..."
until k3s kubectl get nodes | grep -q "Ready"; do
    echo "   等待节点就绪..."
    sleep 5
done

# 设置 kubeconfig
echo "📝 配置 kubectl..."
mkdir -p ~/.kube
cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
chmod 600 ~/.kube/config

# 创建命名空间
echo "📁 创建命名空间..."
k3s kubectl apply -f - <<'NAMESPACE_YAML'
apiVersion: v1
kind: Namespace
metadata:
  name: erp-saas
  labels:
    name: erp-saas
---
apiVersion: v1
kind: Namespace
metadata:
  name: monitoring
  labels:
    name: monitoring
NAMESPACE_YAML

# 安装本地存储 Provisioner
echo "💾 配置本地存储..."
k3s kubectl apply -f - <<'STORAGECLASS_YAML'
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: local-path
  annotations:
    storageclass.kubernetes.io/is-default-class: "true"
provisioner: rancher.io/local-path
reclaimPolicy: Delete
volumeBindingMode: WaitForFirstConsumer
STORAGECLASS_YAML

# 显示状态
echo ""
echo "================================================"
echo "  ✅ K3s 安装完成!"
echo "================================================"
echo ""
echo "节点状态:"
k3s kubectl get nodes
echo ""
echo "系统 Pod 状态:"
k3s kubectl get pods -n kube-system
echo ""
echo "下一步:"
echo "  1. just k3s-deploy   # 部署应用"
echo "  2. k3s kubectl get pods -n erp-saas"
echo ""
