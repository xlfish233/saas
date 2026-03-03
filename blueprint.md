# ERP SaaS Cloud-Native Blueprint (Rust Stack)

> **Version:** 1.0.0
> **Last Updated:** 2026-03-03
> **Target:** AWS EKS + Local k3s/minikube
> **Language:** Rust (Axum)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture Principles](#2-architecture-principles)
3. [System Architecture](#3-system-architecture)
4. [Technology Stack](#4-technology-stack)
5. [Multi-Tenant Architecture](#5-multi-tenant-architecture)
6. [Module System](#6-module-system)
7. [Database Architecture](#7-database-architecture)
8. [Infrastructure](#8-infrastructure)
9. [Local Development](#9-local-development)
10. [CI/CD Pipeline](#10-cicd-pipeline)
11. [Security](#11-security)
12. [Monitoring & Observability](#12-monitoring--observability)
13. [Implementation Roadmap](#13-implementation-roadmap)

---

## 1. Overview

### 1.1 Project Goals

Build a **cloud-native, multi-tenant ERP SaaS platform** with:

- Rust-first technology stack for performance, safety, and reliability
- AWS EKS as primary deployment target
- k3s/minikube for local development with production parity
- Multi-tenant architecture with flexible isolation levels
- Modular design with feature flags and subscription tiers
- Horizontally scalable microservices architecture

### 1.2 Core ERP Modules

| Module | Code | Description |
|--------|------|-------------|
| Finance | `finance` | General ledger, AP/AR, budgeting |
| Inventory | `inventory` | Stock management, warehouses |
| Sales | `sales` | Orders, customers, quotations |
| Procurement | `procurement` | Purchasing, vendors |
| HR | `hr` | Employees, payroll, attendance |
| Production | `production` | Manufacturing, BOM, work orders |
| WMS | `wms` | Warehouse operations |
| Reporting | `reporting` | Analytics, dashboards |

### 1.3 Subscription Tiers

| Tier | Target | Isolation | Resources |
|------|--------|-----------|-----------|
| Starter | SMB | Pool (shared) | 2 CPU, 4GB RAM |
| Pro | Mid-market | Bridge (schema) | 8 CPU, 16GB RAM |
| Enterprise | Large | Silo (dedicated) | 32+ CPU, 64GB+ RAM |

---

## 2. Architecture Principles

### 2.1 Design Principles

```
+-----------------------------------------------------------------+
|                    Architecture Principles                       |
+-----------------------------------------------------------------+
|                                                                 |
|  1. Cloud-Native First                                          |
|     - Containerized workloads                                   |
|     - Kubernetes orchestration                                  |
|     - Service mesh ready                                        |
|                                                                 |
|  2. Multi-Tenant by Design                                      |
|     - Tenant context in every request                           |
|     - Data isolation at database level                          |
|     - Resource quotas per tenant                                |
|                                                                 |
|  3. API-First                                                   |
|     - RESTful + GraphQL APIs                                    |
|     - OpenAPI documentation                                     |
|     - Versioned APIs                                            |
|                                                                 |
|  4. Event-Driven                                                |
|     - Domain events for inter-service communication             |
|     - Event sourcing for audit trails                           |
|     - CQRS for read/write separation                            |
|     - Domain-Driven Design (DDD) bounded contexts               |
|                                                                 |
|  5. Resilience                                                  |
|     - Circuit breakers                                          |
|     - Retry with exponential backoff                            |
|     - Graceful degradation                                      |
|                                                                 |
|  6. Observability                                               |
|     - Structured logging                                        |
|     - Distributed tracing                                       |
|     - Metrics per tenant                                        |
|                                                                 |
|  7. Zero-Trust Security                                         |
|     - Never trust, always verify                                |
|     - mTLS for all service communication                        |
|     - Least privilege access                                    |
|     - Identity-based security perimeter                         |
|                                                                 |
|  8. Infrastructure as Code (IaC)                                |
|     - Declarative infrastructure (Terraform)                    |
|     - GitOps for deployments (ArgoCD)                           |
|     - Immutable infrastructure                                  |
|     - Environment parity (12-Factor App)                        |
|                                                                 |
+-----------------------------------------------------------------+
```

### 2.2 Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language | **Rust** | Performance, memory safety, concurrency |
| Web Framework | **Axum** | Type-safe, async, Tower ecosystem |
| Database | **PostgreSQL (Aurora)** | ACID, RLS, mature ecosystem |
| ORM | **SQLx** | Compile-time checked queries |
| Cache | **Redis** | Performance, pub/sub |
| Message Queue | **NATS JetStream** | Rust-native, high performance |
| Service Mesh | **Istio / Linkerd** | Traffic management, security |
| CI/CD | **GitHub Actions + ArgoCD** | GitOps, automation |

---

## 3. System Architecture

### 3.1 High-Level Architecture

```
+---------------------------------------------------------------------------------+
|                              AWS Cloud                                           |
+---------------------------------------------------------------------------------+
|                                                                                  |
|  +-------------------------------------------------------------------------+    |
|  |                        Route 53 (DNS)                                    |    |
|  |   *.app.com -> ALB    |   api.app.com -> ALB    |   admin.app.com -> ALB|    |
|  +-------------------------------------------------------------------------+    |
|                                       |                                          |
|  +-------------------------------------------------------------------------+    |
|  |                    AWS WAF + CloudFront (CDN)                            |    |
|  +-------------------------------------------------------------------------+    |
|                                       |                                          |
|  +-------------------------------------------------------------------------+    |
|  |                     Application Load Balancer                            |    |
|  |              (Tenant Routing + SSL Termination)                          |    |
|  +-------------------------------------------------------------------------+    |
|                                       |                                          |
|  +-------------------------------------------------------------------------+    |
|  |                          Amazon EKS Cluster                              |    |
|  |                                                                          |    |
|  |   +-------------------------------------------------------------+       |    |
|  |   |              Istio Service Mesh (Ingress Gateway)            |       |    |
|  |   +-------------------------------------------------------------+       |    |
|  |                                    |                                    |    |
|  |   +--------------------------------------------------------------+      |    |
|  |   |                     API Gateway Service                       |      |    |
|  |   |           (Rate Limiting, Auth, Tenant Routing)               |      |    |
|  |   +--------------------------------------------------------------+      |    |
|  |                                    |                                    |    |
|  |   +---------------+  +---------------+  +---------------+              |    |
|  |   |  Namespace    |  |  Namespace    |  |  Namespace    |              |    |
|  |   | tenant-001    |  | tenant-002    |  |   shared      |              |    |
|  |   |  +--------+   |  |  +--------+   |  |  +--------+   |              |    |
|  |   |  |Finance |   |  |  |Finance |   |  |  | Auth   |   |              |    |
|  |   |  |Service |   |  |  |Service |   |  |  |Service |   |              |    |
|  |   |  +--------+   |  |  +--------+   |  |  +--------+   |              |    |
|  |   |  +--------+   |  |  +--------+   |  |  +--------+   |              |    |
|  |   |  |Inventory|  |  |  |Inventory|  |  |  | Tenant |   |              |    |
|  |   |  |Service |   |  |  |Service |   |  |  |Service |   |              |    |
|  |   |  +--------+   |  |  +--------+   |  |  +--------+   |              |    |
|  |   |  +--------+   |  |  +--------+   |  |  +--------+   |              |    |
|  |   |  | Sales  |   |  |  | Sales  |   |  |  |Feature |   |              |    |
|  |   |  |Service |   |  |  |Service |   |  |  |Service |   |              |    |
|  |   |  +--------+   |  |  +--------+   |  |  +--------+   |              |    |
|  |   +---------------+  +---------------+  +---------------+              |    |
|  |                                                                          |    |
|  +-------------------------------------------------------------------------+    |
|                                       |                                          |
|  +-------------------------------------------------------------------------+    |
|  |                           Data Layer                                     |    |
|  |  +-------------+  +-------------+  +-------------+  +---------------+   |    |
|  |  | Aurora PG   |  |ElastiCache  |  |    S3       |  | OpenSearch    |   |    |
|  |  | (Pool/Silo) |  |   Redis     |  |  Storage    |  |    Logs       |   |    |
|  |  +-------------+  +-------------+  +-------------+  +---------------+   |    |
|  +-------------------------------------------------------------------------+    |
|                                                                                  |
+---------------------------------------------------------------------------------+
```

### 3.2 Microservices Architecture

```
+---------------------------------------------------------------------------------+
|                        ERP SaaS Microservices                                    |
+---------------------------------------------------------------------------------+
|                                                                                  |
|  SHARED SERVICES (Global Namespace)                                              |
|  +-----------------+  +-----------------+  +-----------------+                   |
|  |  API Gateway    |  |  Auth Service   |  | Tenant Service  |                   |
|  |  (Kong/APISIX)  |  | (JWT/OAuth2)    |  |  (Provisioning) |                   |
|  +-----------------+  +-----------------+  +-----------------+                   |
|                                                                                  |
|  +-----------------+  +-----------------+  +-----------------+                   |
|  | Feature Service |  | Billing Service |  |Notification Svc |                   |
|  |  (Feature Flags)|  | (Subscription)  |  | (Email/SMS/Push)|                   |
|  +-----------------+  +-----------------+  +-----------------+                   |
|                                                                                  |
|  BUSINESS MODULES (Per-Tenant Namespace)                                         |
|  +-----------------+  +-----------------+  +-----------------+                   |
|  | Finance Service |  |Inventory Service|  |  Sales Service  |                   |
|  |   (财务模块)     |  |   (库存模块)     |  |   (销售模块)     |                   |
|  +-----------------+  +-----------------+  +-----------------+                   |
|                                                                                  |
|  +-----------------+  +-----------------+  +-----------------+                   |
|  |  HR Service     |  |Purchase Service |  |Report Service   |                   |
|  |   (人事模块)     |  |   (采购模块)     |  |   (报表模块)     |                   |
|  +-----------------+  +-----------------+  +-----------------+                   |
|                                                                                  |
|  INFRASTRUCTURE                                                                  |
|  +-----------------+  +-----------------+  +-----------------+                   |
|  | Event Bus       |  | Cache Layer     |  | File Storage    |                   |
|  | (NATS JetStream)|  |    (Redis)      |  |     (S3)        |                   |
|  +-----------------+  +-----------------+  +-----------------+                   |
|                                                                                  |
+---------------------------------------------------------------------------------+
```

---

## 4. Technology Stack

### 4.1 Rust Crate Selection

```toml
# Cargo.toml - Core Dependencies

[dependencies]
# Web Framework
axum = { version = "0.8", features = ["macros", "tower-log"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors", "compression-full"] }
hyper = { version = "1.0", features = ["full"] }

# Async Runtime
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "time", "json"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_with = "3.0"

# Validation
validator = { version = "0.18", features = ["derive"] }

# Authentication
jsonwebtoken = "9.0"
argon2 = "0.5"

# Configuration
config = "0.14"
dotenvy = "0.15"

# Error Handling
thiserror = "1.0"
anyhow = "1.0"

# Logging & Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }

# Metrics
prometheus = "0.13"

# OpenTelemetry (Updated versions)
opentelemetry = { version = "0.27", features = ["metrics", "trace"] }
opentelemetry-otlp = { version = "0.27", features = ["metrics", "trace"] }
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
tracing-opentelemetry = "0.28"

# Redis Client
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }

# AWS SDK
aws-sdk-s3 = "1.0"
aws-sdk-sqs = "1.0"
aws-sdk-secretsmanager = "1.0"
aws-config = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
time = { version = "0.3", features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
rand = "0.8"
regex = "1.10"
async-trait = "0.1"
futures = "0.3"

# Concurrent Data Structures (for rate limiting)
dashmap = "6.0"

# NATS (Message Queue) - Updated
async-nats = "0.46"

# OpenAPI Documentation
utoipa = { version = "5.0", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "8.0", features = ["axum"] }

# HTTP Client
reqwest = { version = "0.12", features = ["json"] }
```

### 4.2 Infrastructure Components

| Component | Production (AWS) | Local Development |
|-----------|------------------|-------------------|
| Kubernetes | Amazon EKS 1.28+ | k3s / minikube |
| Database | Aurora PostgreSQL | PostgreSQL (Docker) |
| Cache | ElastiCache Redis | Redis (Docker) |
| Object Storage | Amazon S3 | MinIO |
| Message Queue | NATS JetStream / SQS | NATS JetStream |
| Secrets | AWS Secrets Manager | Kubernetes Secrets |
| Service Mesh | Istio | (Optional) Linkerd |
| Ingress | ALB + Istio Gateway | Traefik (k3s built-in) |
| Monitoring | CloudWatch + Prometheus | Prometheus + Grafana |
| Logging | OpenSearch | Loki |

### 4.3 Project Structure

```
erp-saas/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── .cargo/
│   └── config.toml               # Cargo config
│
├── crates/
│   ├── api-gateway/              # API Gateway service
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/
│   │       ├── middleware/
│   │       └── handlers/
│   │
│   ├── auth-service/             # Authentication service
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── jwt.rs
│   │       ├── oauth.rs
│   │       └── handlers/
│   │
│   ├── tenant-service/           # Tenant management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── provisioning.rs
│   │       └── handlers/
│   │
│   ├── feature-service/          # Feature flags
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── flags.rs
│   │       └── handlers/
│   │
│   ├── finance-service/          # Finance module
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── domain/
│   │       ├── handlers/
│   │       └── repository/
│   │
│   ├── inventory-service/        # Inventory module
│   │   └── ...
│   │
│   ├── sales-service/            # Sales module
│   │   └── ...
│   │
│   ├── shared/                   # Shared library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config/
│   │       ├── db/
│   │       ├── cache/
│   │       ├── events/
│   │       ├── middleware/
│   │       ├── telemetry/
│   │       └── utils/
│   │
│   └── proto/                    # Protocol definitions
│       ├── Cargo.toml
│       └── src/
│           └── messages.proto
│
├── deploy/
│   ├── base/                     # K8s base manifests
│   │   ├── kustomization.yaml
│   │   ├── namespace.yaml
│   │   └── deployments/
│   │
│   ├── overlays/
│   │   ├── local/                # Local development
│   │   │   ├── kustomization.yaml
│   │   │   └── patches/
│   │   │
│   │   ├── staging/              # Staging environment
│   │   │   └── ...
│   │   │
│   │   └── production/           # Production environment
│   │       └── ...
│   │
│   └── local/                    # Local infrastructure
│       ├── docker-compose.yaml
│       ├── k3s-setup.sh
│       └── minio-setup.sh
│
├── infrastructure/
│   ├── terraform/                # AWS infrastructure
│   │   ├── main.tf
│   │   ├── vpc.tf
│   │   ├── eks.tf
│   │   ├── rds.tf
│   │   └── variables.tf
│   │
│   └── localstack/               # Local AWS simulation
│       └── init.sh
│
├── migrations/                   # Database migrations
│   ├── 20240101000000_init.up.sql
│   └── 20240101000000_init.down.sql
│
├── scripts/
│   ├── dev-start.sh              # Start local environment
│   ├── dev-stop.sh               # Stop local environment
│   └── seed-data.sh              # Seed test data
│
├── .github/
│   └── workflows/
│       ├── ci.yaml               # CI pipeline
│       └── deploy.yaml           # CD pipeline
│
├── Makefile                      # Common commands
├── Dockerfile                    # Multi-stage build
├── docker-compose.yaml           # Local development
└── README.md
```

---

## 5. Multi-Tenant Architecture

### 5.1 Tenant Isolation Models

```
+-----------------------------------------------------------------+
|                    Isolation Model Spectrum                      |
+-----------------------------------------------------------------+
|                                                                 |
|  Pool Model (Starter Tier)                                      |
|  +----------------------------------------------------------+   |
|  |  Shared Database, Shared Schema                          |   |
|  |  +----------------------------------------------------+  |   |
|  |  |              orders table                          |  |   |
|  |  |  id | tenant_id | customer | amount | ...          |  |   |
|  |  |  1  | T001      | C001     | 100    | ...          |  |   |
|  |  |  2  | T002      | C002     | 200    | ...          |  |   |
|  |  +----------------------------------------------------+  |   |
|  |  Isolation: Row-Level Security (RLS)                    |   |
|  +----------------------------------------------------------+   |
|                                                                 |
|  Bridge Model (Pro Tier)                                        |
|  +----------------------------------------------------------+   |
|  |  Shared Database, Separate Schemas                       |   |
|  |  +----------------+  +----------------+                  |   |
|  |  | tenant_001     |  | tenant_002     |                  |   |
|  |  | +-----------+  |  | +-----------+  |                  |   |
|  |  | | orders    |  |  | | orders    |  |                  |   |
|  |  | | products  |  |  | | products  |  |                  |   |
|  |  | +-----------+  |  | +-----------+  |                  |   |
|  |  +----------------+  +----------------+                  |   |
|  |  Isolation: Schema-level permissions                    |   |
|  +----------------------------------------------------------+   |
|                                                                 |
|  Silo Model (Enterprise Tier)                                   |
|  +----------------------------------------------------------+   |
|  |  Separate Database / Cluster                             |   |
|  |  +----------------+  +----------------+                  |   |
|  |  | tenant_001_db  |  | tenant_002_db  |                  |   |
|  |  | (Aurora Cls 1) |  | (Aurora Cls 2) |                  |   |
|  |  +----------------+  +----------------+                  |   |
|  |  Isolation: Physical separation                          |   |
|  +----------------------------------------------------------+   |
|                                                                 |
+-----------------------------------------------------------------+
```

### 5.2 Tenant Context Propagation

```rust
// crates/shared/src/tenant/mod.rs
use uuid::Uuid;
use axum::{
    extract::{FromRequestParts, Request},
    http::{header, request::Parts},
    middleware::Next,
    response::Response,
};

#[derive(Clone, Debug)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub tier: TenantTier,
    pub features: Vec<String>,
    pub limits: TenantLimits,
}

#[derive(Clone, Debug, strum::Display)]
pub enum TenantTier {
    Starter,
    Pro,
    Enterprise,
}

#[derive(Clone, Debug)]
pub struct TenantLimits {
    pub max_users: u32,
    pub max_storage_gb: u32,
    pub rate_limit_per_minute: u32,
}

// Extract tenant from JWT claims or subdomain
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = axum::response::ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Try to get from JWT claims
        if let Some(claims) = parts.extensions.get::<JwtClaims>() {
            return Ok(claims.tenant_context.clone());
        }

        // 2. Try to get from subdomain
        let host = parts
            .headers
            .get(header::HOST)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Extract subdomain: tenant-001.app.com
        if let Some(tenant_id) = extract_tenant_from_subdomain(host) {
            // Load tenant context from cache/database
            let ctx = load_tenant_context(tenant_id).await?;
            return Ok(ctx);
        }

        Err(axum::http::StatusCode::BAD_REQUEST.into())
    }
}

// Middleware to inject tenant context
pub async fn tenant_middleware(
    req: Request,
    next: Next,
) -> Response {
    // Extract and set tenant context
    // ...
    next.run(req).await
}
```

### 5.3 Kubernetes Namespace Isolation

```yaml
# Tenant Namespace Template
apiVersion: v1
kind: Namespace
metadata:
  name: tenant-001
  labels:
    tenant-id: "001"
    tier: "pro"
    isolation: "bridge"
---
# Resource Quota per tenant
apiVersion: v1
kind: ResourceQuota
metadata:
  name: tenant-quota
  namespace: tenant-001
spec:
  hard:
    requests.cpu: "8"
    requests.memory: 16Gi
    limits.cpu: "16"
    limits.memory: 32Gi
    pods: "50"
---
# Network Policy - Default Deny
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-all
  namespace: tenant-001
spec:
  podSelector: {}
  policyTypes:
    - Ingress
    - Egress
---
# Network Policy - Allow intra-namespace
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-intra-namespace
  namespace: tenant-001
spec:
  podSelector: {}
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: tenant-001
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              name: tenant-001
---
# Network Policy - Allow shared services
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-shared-services
  namespace: tenant-001
spec:
  podSelector: {}
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              name: shared-services
---
# Network Policy - Allow DNS resolution (CRITICAL)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-dns
  namespace: tenant-001
spec:
  podSelector: {}
  policyTypes:
    - Egress
  egress:
    - to:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: kube-system
      ports:
        - protocol: UDP
          port: 53
        - protocol: TCP
          port: 53
---
# Network Policy - Allow ingress from ALB/Istio Gateway
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-ingress-traffic
  namespace: tenant-001
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/part-of: erp-saas
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: istio-system
      ports:
        - protocol: TCP
          port: 8080
---
# Network Policy - Allow database egress (Aurora/RDS)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-database-access
  namespace: tenant-001
spec:
  podSelector: {}
  policyTypes:
    - Egress
  egress:
    - to:
        - ipBlock:
            cidr: 10.0.64.0/24  # Database subnet CIDR
      ports:
        - protocol: TCP
          port: 5432  # PostgreSQL
---
# Network Policy - Allow monitoring (Prometheus scraping)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-monitoring
  namespace: tenant-001
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/part-of: erp-saas
  policyTypes:
    - Ingress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: monitoring
      ports:
        - protocol: TCP
          port: 9090
---
# Network Policy - Allow external HTTPS egress (controlled)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-external-https
  namespace: tenant-001
spec:
  podSelector: {}
  policyTypes:
    - Egress
  egress:
    - to:
        - ipBlock:
            cidr: 0.0.0.0/0
            except:
              - 10.0.0.0/8      # Block internal IPs
              - 172.16.0.0/12
              - 192.168.0.0/16
      ports:
        - protocol: TCP
          port: 443
---
# Namespace with Pod Security Standards
apiVersion: v1
kind: Namespace
metadata:
  name: tenant-001
  labels:
    tenant-id: "001"
    tier: "pro"
    isolation: "bridge"
    pod-security.kubernetes.io/enforce: restricted
    pod-security.kubernetes.io/audit: restricted
    pod-security.kubernetes.io/warn: restricted
```

---

## 6. Module System

### 6.1 Module Architecture

```
+-----------------------------------------------------------------+
|                        Module System                             |
+-----------------------------------------------------------------+
|                                                                 |
|  Core Modules (Always Available)                                |
|  +----------------+  +----------------+  +----------------+      |
|  | Auth Module    |  | Tenant Module  |  | Billing Module |      |
|  | - Login/Logout |  | - Provisioning |  | - Subscription |      |
|  | - User Mgmt    |  | - Settings     |  | - Invoices     |      |
|  +----------------+  +----------------+  +----------------+      |
|                                                                 |
|  Business Modules (Feature Flagged)                             |
|  +----------------+  +----------------+  +----------------+      |
|  | Finance        |  | Inventory      |  | Sales          |      |
|  | - GL           |  | - Stock        |  | - Orders       |      |
|  | - AP/AR        |  | - Warehouses   |  | - Customers    |      |
|  | - Budgeting    |  | - Movements    |  | - Quotations   |      |
|  +----------------+  +----------------+  +----------------+      |
|                                                                 |
|  +----------------+  +----------------+  +----------------+      |
|  | HR             |  | Procurement    |  | Production     |      |
|  | - Employees    |  | - PO           |  | - BOM          |      |
|  | - Payroll      |  | - Vendors      |  | - Work Orders  |      |
|  | - Attendance   |  | - Receiving    |  | - Routing      |      |
|  +----------------+  +----------------+  +----------------+      |
|                                                                 |
|  Premium Features (Enterprise Only)                             |
|  +----------------+  +----------------+                          |
|  | AI Forecasting |  | Advanced Rpt   |                          |
|  +----------------+  +----------------+                          |
|                                                                 |
+-----------------------------------------------------------------+
```

### 6.2 Feature Flag System

```rust
// crates/feature-service/src/flags.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub features: HashMap<String, FeatureConfig>,
    pub tier_features: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub enabled: bool,
    pub rollout_percentage: u8,
    pub required_tier: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TenantFeatures {
    pub tenant_id: Uuid,
    pub enabled_features: HashSet<String>,
    pub limits: TierLimits,
}

impl FeatureFlags {
    /// Check if a feature is enabled for a tenant
    pub fn is_enabled(
        &self,
        feature: &str,
        tenant_id: &Uuid,
        tier: &str,
    ) -> bool {
        let Some(config) = self.features.get(feature) else {
            return false;
        };

        if !config.enabled {
            return false;
        }

        // Check tier requirement
        if let Some(required) = &config.required_tier {
            if !self.tier_satisfies(tier, required) {
                return false;
            }
        }

        // Check dependencies
        for dep in &config.dependencies {
            if !self.is_enabled(dep, tenant_id, tier) {
                return false;
            }
        }

        // Check rollout percentage
        if config.rollout_percentage < 100 {
            let hash = Self::hash_tenant_feature(tenant_id, feature);
            let bucket = hash % 100;
            if bucket >= config.rollout_percentage as u64 {
                return false;
            }
        }

        true
    }

    fn tier_satisfies(&self, tenant_tier: &str, required: &str) -> bool {
        let tier_order = ["starter", "pro", "enterprise"];
        let tenant_idx = tier_order.iter().position(|&t| t == tenant_tier);
        let required_idx = tier_order.iter().position(|&t| t == required);

        match (tenant_idx, required_idx) {
            (Some(t), Some(r)) => t >= r,
            _ => false,
        }
    }

    fn hash_tenant_feature(tenant_id: &Uuid, feature: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tenant_id.hash(&mut hasher);
        feature.hash(&mut hasher);
        hasher.finish()
    }
}
```

### 6.3 Feature Configuration

```json
// config/feature-flags.json
{
  "features": {
    "finance": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "starter"
    },
    "finance.advanced_budgeting": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "pro",
      "dependencies": ["finance"]
    },
    "inventory": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "starter"
    },
    "inventory.multi_warehouse": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "pro",
      "dependencies": ["inventory"]
    },
    "sales": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "starter"
    },
    "hr": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "pro"
    },
    "production": {
      "enabled": true,
      "rollout_percentage": 100,
      "required_tier": "enterprise"
    },
    "ai_forecasting": {
      "enabled": true,
      "rollout_percentage": 50,
      "required_tier": "enterprise"
    }
  },
  "tier_limits": {
    "starter": {
      "max_users": 5,
      "max_storage_gb": 10,
      "rate_limit_per_minute": 100
    },
    "pro": {
      "max_users": 50,
      "max_storage_gb": 100,
      "rate_limit_per_minute": 500
    },
    "enterprise": {
      "max_users": -1,
      "max_storage_gb": -1,
      "rate_limit_per_minute": 2000
    }
  }
}
```

---

## 7. Database Architecture

### 7.1 Aurora PostgreSQL Configuration

```hcl
# infrastructure/terraform/rds.tf
resource "aws_rds_cluster" "main" {
  cluster_identifier              = "erp-saas-aurora"
  engine                          = "aurora-postgresql"
  engine_version                  = "15.4"
  database_name                   = "erp"
  master_username                 = "admin"
  manage_master_user_password     = true

  # Serverless v2 scaling - Updated min capacity for production stability
  serverlessv2_scaling_configuration {
    min_capacity = 4    # Minimum 4 ACU for production workloads
    max_capacity = 64
  }

  # Encryption
  storage_encrypted               = true
  kms_key_id                      = aws_kms_key.rds.arn

  # Backup
  backup_retention_period         = 35
  preferred_backup_window         = "03:00-04:00"
  skip_final_snapshot             = false
  final_snapshot_identifier       = "erp-saas-final-snapshot"

  # Monitoring
  enabled_cloudwatch_logs_exports = ["postgresql"]

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

# RDS Proxy for connection pooling and security
resource "aws_db_proxy" "main" {
  name                   = "erp-saas-proxy"
  engine_family          = "POSTGRESQL"
  require_tls            = true
  
  auth {
    auth_scheme = "SECRETS"
    iam_auth    = "REQUIRED"
    secret_arn  = aws_secretsmanager_secret.db_credentials.arn
  }

  db_proxy_target_role {
    role_arn = aws_iam_role.rds_proxy.arn
  }

  vpc_subnet_ids = module.vpc.private_subnets
  vpc_security_group_ids = [aws_security_group.rds_proxy.id]

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

resource "aws_db_proxy_default_target" "main" {
  db_proxy_name          = aws_db_proxy.main.name
  target_group_name      = "default"
  db_cluster_identifier  = aws_rds_cluster.main.id
  
  connection_pool_config {
    connection_borrow_timeout = 120
    max_connections_percent  = 100
    session_pinning_filters  = ["EXCLUDE_VARIABLE_SETS"]
  }
}

# IAM role for RDS Proxy
resource "aws_iam_role" "rds_proxy" {
  name = "erp-saas-rds-proxy-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action = "sts:AssumeRole"
      Effect = "Allow"
      Principal = {
        Service = "rds.amazonaws.com"
      }
    }]
  })
}

# Security group for RDS Proxy
resource "aws_security_group" "rds_proxy" {
  name        = "erp-saas-rds-proxy-sg"
  description = "Security group for RDS Proxy"
  vpc_id      = module.vpc.vpc_id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    security_groups = [module.eks.node_security_group_id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

resource "aws_rds_cluster_instance" "main" {
  count              = 2
  identifier         = "erp-saas-${count.index}"
  cluster_identifier = aws_rds_cluster.main.id
  instance_class     = "db.serverless"
  engine             = aws_rds_cluster.main.engine
  engine_version     = aws_rds_cluster.main.engine_version

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}
```

### 7.2 Row-Level Security (Pool Model)

```sql
-- migrations/20240101000001_rls.up.sql

-- Enable RLS extension
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Create tenant context function
CREATE OR REPLACE FUNCTION set_tenant_context(tenant_id UUID)
RETURNS VOID AS $$
BEGIN
    EXECUTE format('SET app.current_tenant = %L', tenant_id::TEXT);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Apply RLS to tables
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE products ENABLE ROW LEVEL SECURITY;
ALTER TABLE customers ENABLE ROW LEVEL SECURITY;
ALTER TABLE inventory_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE transactions ENABLE ROW LEVEL SECURITY;

-- Create policies
CREATE POLICY tenant_isolation_policy ON orders
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

CREATE POLICY tenant_isolation_policy ON products
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

CREATE POLICY tenant_isolation_policy ON customers
    USING (tenant_id = current_setting('app.current_tenant')::UUID);

-- Force RLS for all roles
ALTER TABLE orders FORCE ROW LEVEL SECURITY;
ALTER TABLE products FORCE ROW LEVEL SECURITY;
ALTER TABLE customers FORCE ROW LEVEL SECURITY;
```

### 7.3 Schema-Per-Tenant (Bridge Model)

```sql
-- migrations/20240101000002_tenant_schema.up.sql

-- Create tenant schema template
CREATE OR REPLACE FUNCTION create_tenant_schema(tenant_id UUID)
RETURNS VOID AS $$
DECLARE
    schema_name TEXT := 'tenant_' || REPLACE(tenant_id::TEXT, '-', '_');
BEGIN
    -- Create schema
    EXECUTE format('CREATE SCHEMA IF NOT EXISTS %I', schema_name);

    -- Create tables in tenant schema
    EXECUTE format('
        CREATE TABLE IF NOT EXISTS %I.orders (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            customer_id UUID NOT NULL,
            total_amount DECIMAL(12,2) NOT NULL,
            status VARCHAR(50) NOT NULL DEFAULT ''pending'',
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )', schema_name);

    EXECUTE format('
        CREATE TABLE IF NOT EXISTS %I.products (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            sku VARCHAR(100) NOT NULL,
            name VARCHAR(255) NOT NULL,
            price DECIMAL(12,2) NOT NULL,
            stock_quantity INTEGER DEFAULT 0,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(sku)
        )', schema_name);

    -- Create tenant role
    EXECUTE format('CREATE ROLE %I_tenant_role', schema_name);
    EXECUTE format('GRANT USAGE ON SCHEMA %I TO %I_tenant_role', schema_name, schema_name);
    EXECUTE format('GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA %I TO %I_tenant_role', schema_name, schema_name);

    -- Set default privileges
    EXECUTE format('ALTER DEFAULT PRIVILEGES IN SCHEMA %I GRANT ALL ON TABLES TO %I_tenant_role', schema_name, schema_name);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
```

### 7.4 Rust Database Layer

```rust
// crates/shared/src/db/mod.rs
use sqlx::{PgPool, Postgres, query_as};
use uuid::Uuid;
use crate::tenant::TenantContext;

pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Execute query with tenant context
    pub async fn with_tenant<F, T>(&self, tenant: &TenantContext, f: F) -> Result<T, sqlx::Error>
    where
        F: for<'c> FnOnce(&'c mut sqlx::PgConnection) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, sqlx::Error>> + 'c>>,
    {
        let mut conn = self.pool.acquire().await?;

        // Set tenant context for RLS
        sqlx::query("SELECT set_tenant_context($1)")
            .bind(tenant.tenant_id)
            .execute(&mut *conn)
            .await?;

        // Execute query
        let result = f(&mut conn).await?;

        // Reset context
        sqlx::query("RESET app.current_tenant")
            .execute(&mut *conn)
            .await?;

        Ok(result)
    }

    /// Execute query in tenant schema (Bridge model)
    pub async fn with_schema<F, T>(&self, tenant: &TenantContext, f: F) -> Result<T, sqlx::Error>
    where
        F: for<'c> FnOnce(&'c mut sqlx::PgConnection, &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, sqlx::Error>> + 'c>>,
    {
        let mut conn = self.pool.acquire().await?;
        let schema = format!("tenant_{}", tenant.tenant_id.to_string().replace('-', "_"));

        // Set search path
        sqlx::query(&format!("SET search_path TO {}", schema))
            .execute(&mut *conn)
            .await?;

        // Execute query
        let result = f(&mut *conn, &schema).await?;

        // Reset search path
        sqlx::query("SET search_path TO public")
            .execute(&mut *conn)
            .await?;

        Ok(result)
    }
}

// Example repository
pub struct OrderRepository {
    db: Database,
}

impl OrderRepository {
    pub async fn find_by_id(
        &self,
        tenant: &TenantContext,
        order_id: Uuid,
    ) -> Result<Option<Order>, sqlx::Error> {
        self.db.with_tenant(tenant, |conn| {
            Box::pin(async move {
                sqlx::query_as::<_, Order>(
                    "SELECT * FROM orders WHERE id = $1"
                )
                .bind(order_id)
                .fetch_optional(conn)
                .await
            })
        }).await
    }

    pub async fn create(
        &self,
        tenant: &TenantContext,
        order: CreateOrder,
    ) -> Result<Order, sqlx::Error> {
        self.db.with_tenant(tenant, |conn| {
            Box::pin(async move {
                sqlx::query_as::<_, Order>(
                    r#"
                    INSERT INTO orders (customer_id, total_amount, status)
                    VALUES ($1, $2, $3)
                    RETURNING *
                    "#
                )
                .bind(order.customer_id)
                .bind(order.total_amount)
                .bind(order.status)
                .fetch_one(conn)
                .await
            })
        }).await
    }
}
```

---

## 8. Infrastructure

### 8.1 AWS VPC Architecture

```hcl
# infrastructure/terraform/vpc.tf
module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "5.0.0"

  name = "erp-saas-vpc"
  cidr = "10.0.0.0/16"

  azs              = ["us-east-1a", "us-east-1b", "us-east-1c"]
  private_subnets  = ["10.0.128.0/18", "10.0.192.0/18", "10.0.224.0/18"]
  public_subnets   = ["10.0.0.0/20", "10.0.16.0/20", "10.0.32.0/20"]
  database_subnets = ["10.0.64.0/24", "10.0.65.0/24", "10.0.66.0/24"]

  # NAT Gateway for private subnet internet access
  enable_nat_gateway     = true
  single_nat_gateway     = false
  one_nat_gateway_per_az = true

  # VPC Flow Logs
  enable_flow_log                      = true
  create_flow_log_cloudwatch_log_group = true
  create_flow_log_cloudwatch_iam_role  = true

  # Tags
  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }

  public_subnet_tags = {
    Type = "public"
  }

  private_subnet_tags = {
    Type     = "private"
    "kubernetes.io/role/internal-elb" = "1"
  }

  database_subnet_tags = {
    Type = "database"
  }
}
```

### 8.2 EKS Cluster

```hcl
# infrastructure/terraform/eks.tf
module "eks" {
  source  = "terraform-aws-modules/eks/aws"
  version = "19.15.0"

  cluster_name    = "erp-saas-cluster"
  cluster_version = "1.28"

  vpc_id     = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnets

  # OIDC Identity Provider (for IRSA)
  enable_irsa = true

  # Cluster encryption
  cluster_encryption_config = {
    provider_key_arn = aws_kms_key.eks.arn
    resources        = ["secrets"]
  }

  # Managed Node Groups
  eks_managed_node_groups = {
    # System workloads
    system = {
      min_size     = 2
      max_size     = 5
      desired_size = 3

      instance_types = ["m6i.xlarge"]
      capacity_type  = "ON_DEMAND"

      labels = {
        workload-type = "system"
      }

      taints = {
        system = {
          key    = "system"
          value  = "true"
          effect = "NO_SCHEDULE"
        }
      }
    }

    # Tenant workloads
    tenants = {
      min_size     = 3
      max_size     = 50
      desired_size = 5

      instance_types = ["m6i.xlarge", "m6i.2xlarge"]
      capacity_type  = "MIXED"

      labels = {
        workload-type = "tenant"
      }
    }
  }

  # Karpenter for autoscaling
  enable_karpenter = true

  # Cluster endpoint access control (security best practice)
  cluster_endpoint_public_access  = false  # Disable public endpoint
  cluster_endpoint_private_access = true   # Private access only

  cluster_endpoint_public_access_cidrs = [] # No public access CIDRs

  # Control plane logging
  enabled_cluster_log_types = ["api", "audit", "authenticator", "controllerManager", "scheduler"]

  # Cluster add-ons
  cluster_addons = {
    coredns = {
      most_recent = true
    }
    kube-proxy = {
      most_recent = true
    }
    vpc-cni = {
      most_recent = true
    }
    aws-ebs-csi-driver = {
      most_recent = true
    }
    # Secrets Store CSI Driver
    secrets-store-csi-driver = {
      most_recent = true
    }
  }

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}
```

### 8.3 ElastiCache Redis

```hcl
# infrastructure/terraform/elasticache.tf
resource "aws_elasticache_replication_group" "main" {
  replication_group_id = "erp-saas-redis"
  description          = "ERP SaaS Redis Cluster"

  engine               = "redis"
  engine_version       = "7.0"
  node_type            = "cache.r6g.xlarge"
  num_cache_clusters   = 3
  port                 = 6379

  # Cluster mode
  cluster_mode {
    replicas_per_node_group = 2
    num_node_groups         = 3
  }

  # Multi-AZ
  multi_az_enabled = true
  automatic_failover_enabled = true

  # Security
  security_group_ids = [aws_security_group.redis.id]
  subnet_group_name  = aws_elasticache_subnet_group.main.name

  # Encryption
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  auth_token                 = random_password.redis_auth.result

  # Backup
  snapshot_retention_limit = 7
  snapshot_window         = "02:00-03:00"

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

resource "aws_elasticache_subnet_group" "main" {
  name       = "erp-saas-redis"
  subnet_ids = module.vpc.private_subnets
}
```

### 8.4 Additional AWS Resources (WAF, Route 53, ACM, S3)

```hcl
# infrastructure/terraform/waf.tf
resource "aws_wafv2_web_acl" "main" {
  name        = "erp-saas-waf-acl"
  description = "WAF ACL for ERP SaaS Application"
  scope       = "REGIONAL"

  default_action {
    allow {}
  }

  # AWS Managed Rules
  rule {
    name     = "AWSManagedRulesCommonRuleSet"
    priority = 1
    override_action {
      none {}
    }
    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesCommonRuleSet"
        vendor_name = "AWS"
      }
    }
    visibility_config {
      cloudwatch_metrics_enabled = true
      sampled_requests_enabled   = true
    }
  }

  # SQL Injection Protection
  rule {
    name     = "AWSManagedRulesSQLiRuleSet"
    priority = 2
    override_action {
      none {}
    }
    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesSQLiRuleSet"
        vendor_name = "AWS"
      }
    }
    visibility_config {
      cloudwatch_metrics_enabled = true
      sampled_requests_enabled   = true
    }
  }

  # Rate Limiting
  rule {
    name     = "RateLimitRule"
    priority = 3
    override_action {
      none {}
    }
    statement {
      rate_based_statement {
        limit              = 10000
        aggregate_key_type = "IP"
      }
    }
    visibility_config {
      cloudwatch_metrics_enabled = true
      sampled_requests_enabled   = true
    }
  }

  visibility_config {
    cloudwatch_metrics_enabled = true
    sampled_requests_enabled   = true
  }

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

# Associate WAF with ALB
resource "aws_wafv2_web_acl_association" "main" {
  resource_arn = aws_lb.main.arn
  web_acl_arn  = aws_wafv2_web_acl.main.arn
}

# infrastructure/terraform/route53.tf
resource "aws_route53_zone" "main" {
  name          = "erp-saas.example.com"
  comment       = "Primary DNS zone for ERP SaaS"
  force_destroy = false

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

# Route 53 A Record for API (Alias to ALB)
resource "aws_route53_record" "api" {
  zone_id = aws_route53_zone.main.zone_id
  name    = "api.erp-saas.example.com"
  type    = "A"

  alias {
    name                   = aws_lb.main.dns_name
    zone_id                = aws_lb.main.zone_id
    evaluate_target_health = true
  }
}

# Route 53 A Record for App
resource "aws_route53_record" "app" {
  zone_id = aws_route53_zone.main.zone_id
  name    = "app.erp-saas.example.com"
  type    = "A"

  alias {
    name                   = aws_lb.main.dns_name
    zone_id                = aws_lb.main.zone_id
    evaluate_target_health = true
  }
}

# infrastructure/terraform/acm.tf
resource "aws_acm_certificate" "main" {
  domain       = "erp-saas.example.com"
  subject_alternative_names = [
    "*.erp-saas.example.com"
  ]
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }

  tags = {
    Environment = "production"
    Project     = "erp-saas"
  }
}

# DNS validation record
resource "aws_route53_record" "cert_validation" {
  for_each = {
    for dvo in aws_acm_certificate.main.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = aws_route53_zone.main.zone_id
}

resource "aws_acm_certificate_validation" "main" {
  certificate_arn         = aws_acm_certificate.main.arn
  validation_record_fqdns = [for record in aws_route53_record.cert_validation : record.fqdn]
}

# infrastructure/terraform/s3.tf
# S3 Bucket for Static Assets
resource "aws_s3_bucket" "assets" {
  bucket = "erp-saas-assets-${random_id.bucket_suffix.hex}"

  tags = {
    Environment = "production"
    Project     = "erp-saas"
    Name        = "assets"
  }
}

resource "aws_s3_bucket_versioning" "assets" {
  bucket = aws_s3_bucket.assets.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "assets" {
  bucket = aws_s3_bucket.assets.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "assets" {
  bucket = aws_s3_bucket.assets.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

# S3 Bucket for Document Storage (Tenant Files)
resource "aws_s3_bucket" "documents" {
  bucket = "erp-saas-documents-${random_id.bucket_suffix.hex}"

  tags = {
    Environment = "production"
    Project     = "erp-saas"
    Name        = "documents"
  }
}

resource "aws_s3_bucket_versioning" "documents" {
  bucket = aws_s3_bucket.documents.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "documents" {
  bucket = aws_s3_bucket.documents.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm     = "aws:kms"
      kms_master_key_id = aws_kms_key.s3.arn
    }
  }
}

# S3 Bucket for Terraform State
resource "aws_s3_bucket" "terraform_state" {
  bucket = "erp-saas-terraform-state-${random_id.bucket_suffix.hex}"

  tags = {
    Environment = "production"
    Project     = "erp-saas"
    Name        = "terraform-state"
  }
}

resource "aws_s3_bucket_versioning" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "terraform_state" {
  bucket = aws_s3_bucket.terraform_state.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "random_id" "bucket_suffix" {
  byte_length = 4
}
```

---

## 9. Local Development

### 9.1 Docker Compose Stack

```yaml
# docker-compose.yaml
version: "3.8"

services:
  postgres:
    image: postgres:15-alpine
    container_name: erp-postgres
    environment:
      POSTGRES_USER: erp
      POSTGRES_PASSWORD: dev_password
      POSTGRES_DB: erp
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U erp"]
      interval: 5s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    container_name: erp-redis
    ports:
      - "6379:6379"
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 5s
      retries: 5

  minio:
    image: minio/minio:latest
    container_name: erp-minio
    environment:
      MINIO_ROOT_USER: local_dev_user
      MINIO_ROOT_PASSWORD: local_dev_secret
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data
    command: server /data --console-address ":9001"

  nats:
    image: nats:2-alpine
    container_name: erp-nats
    ports:
      - "4222:4222"
      - "8222:8222"
    command: "--jetstream --store_dir /data --http_port 8222"
    volumes:
      - nats_data:/data

  localstack:
    image: localstack/localstack:latest
    container_name: erp-localstack
    environment:
      SERVICES: sqs,sns,secretsmanager
      AWS_DEFAULT_REGION: us-east-1
      EDGE_PORT: 4566
    ports:
      - "4566:4566"
    volumes:
      - localstack_data:/var/lib/localstack
      - /var/run/docker.sock:/var/run/docker.sock

  mailhog:
    image: mailhog/mailhog:latest
    container_name: erp-mailhog
    ports:
      - "1025:1025"
      - "8025:8025"

volumes:
  postgres_data:
  redis_data:
  minio_data:
  nats_data:
  localstack_data:
```

### 9.2 k3s Setup Script

```bash
#!/bin/bash
# scripts/k3s-setup.sh

set -e

echo "Installing k3s..."

# Install k3s with Traefik disabled (we'll use our own ingress)
curl -sfL https://get.k3s.io | sh -s - \
  --disable traefik \
  --write-kubeconfig-mode 644

# Wait for k3s to be ready
echo "Waiting for k3s to be ready..."
sleep 10

# Create namespaces
kubectl create namespace erp-local
kubectl create namespace shared-services

# Install metrics server
kubectl apply -f https://github.com/kubernetes-sigs/metrics-server/releases/latest/download/components.yaml

# Install nginx ingress
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.8.2/deploy/static/provider/cloud/deploy.yaml

echo "k3s setup complete!"
echo "Kubeconfig: /etc/rancher/k3s/k3s.yaml"
```

### 9.3 Local Development Script

```bash
#!/bin/bash
# scripts/dev-start.sh

set -e

echo "Starting ERP SaaS Local Development Environment..."

# 1. Start infrastructure
echo "Starting infrastructure services..."
docker compose up -d

# Wait for services
echo "Waiting for services to be healthy..."
sleep 10

# 2. Run migrations
echo "Running database migrations..."
cargo sqlx migrate run

# 3. Initialize MinIO
echo "Initializing MinIO..."
docker exec erp-minio mc alias set local http://localhost:9000 local_dev_user local_dev_secret 2>/dev/null || true
docker exec erp-minio mc mb local/erp-files 2>/dev/null || true

# 4. Build and run services
echo "Building services..."
cargo build --release

# 5. Port forwarding info
echo ""
echo "=================================="
echo "Local environment is ready!"
echo "=================================="
echo ""
echo "Services:"
echo "  PostgreSQL:  localhost:5432"
echo "  Redis:       localhost:6379"
echo "  MinIO API:   http://localhost:9000"
echo "  MinIO UI:    http://localhost:9001"
echo "  NATS:        localhost:4222"
echo "  NATS UI:     http://localhost:8222"
echo "  LocalStack:  http://localhost:4566"
echo "  MailHog:     http://localhost:8025"
echo ""
echo "To start API server:"
echo "  cargo run --bin api-gateway"
echo ""
```

### 9.4 Makefile

```makefile
# Makefile

.PHONY: help dev build test clean

help:
	@echo "ERP SaaS Development Commands"
	@echo ""
	@echo "Development:"
	@echo "  dev-up       - Start local infrastructure"
	@echo "  dev-down     - Stop local infrastructure"
	@echo "  dev-clean    - Clean all local data"
	@echo ""
	@echo "Building:"
	@echo "  build        - Build all services"
	@echo "  build-rel    - Build release version"
	@echo ""
	@echo "Database:"
	@echo "  db-migrate   - Run migrations"
	@echo "  db-reset     - Reset database"
	@echo "  db-seed      - Seed test data"
	@echo ""
	@echo "Testing:"
	@echo "  test         - Run unit tests"
	@echo "  test-e2e     - Run e2e tests"
	@echo ""
	@echo "Docker:"
	@echo "  docker-build - Build Docker images"
	@echo "  k8s-deploy   - Deploy to local k8s"

# Development
dev-up:
	docker compose up -d
	./scripts/dev-start.sh

dev-down:
	docker compose down

dev-clean:
	docker compose down -v
	rm -rf target/

# Building
build:
	cargo build

build-rel:
	cargo build --release

# Database
db-migrate:
	cargo sqlx migrate run

db-reset:
	cargo sqlx database reset -y

db-seed:
	cargo run --bin seed-data

# Testing
test:
	cargo test --all-features

test-e2e:
	cargo test --test e2e -- --test-threads=1

# Docker
docker-build:
	docker build -t erp-saas:latest .

k8s-deploy:
	kubectl apply -k deploy/overlays/local
```

---

## 10. CI/CD Pipeline

### 10.1 GitHub Actions Workflow

```yaml
# .github/workflows/ci.yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  lint-and-test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: test
          POSTGRES_PASSWORD: test
          POSTGRES_DB: test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

      redis:
        image: redis:7
        ports:
          - 6379:6379

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Run tests
        run: cargo test --all-features
        env:
          DATABASE_URL: postgres://test:test@localhost:5432/test
          REDIS_URL: redis://localhost:6379

  # SAST - Static Application Security Testing
  sast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Semgrep SAST Scan
        uses: returntocorp/semgrep-action@v1
        with:
          config: >-
            rules:
              - p/security-audit
              - p/secrets
          generateSarifFile: true

      - name: Upload SARIF to GitHub Security
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: semgrep.sarif

  # Secret Scanning
  secret-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Gitleaks Secret Scan
        uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  # Dependency Security Audit
  security-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cargo Audit
        run: |
          cargo install cargo-audit
          cargo audit

  # Build and Scan Container Image
  build-and-scan:
    needs: [lint-and-test, sast, secret-scan, security-audit]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1

      - name: Login to ECR
        id: login-ecr
        uses: aws-actions/amazon-ecr-login@v2

      - name: Build Docker image
        env:
          ECR_REGISTRY: ${{ steps.login-ecr.outputs.registry }}
          IMAGE_TAG: ${{ github.sha }}
        run: |
          docker build -t $ECR_REGISTRY/erp-saas:$IMAGE_TAG .

      - name: Trivy Container Vulnerability Scan
        uses: aquasecurity/trivy-action@master
        with:
          image-ref: ${{ steps.login-ecr.outputs.registry }}/erp-saas:${{ github.sha }}
          format: 'sarif'
          output: 'trivy-results.sarif'
          severity: 'CRITICAL,HIGH'
          ignore-unfixed: true

      - name: Upload Trivy SARIF to GitHub Security
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: trivy-results.sarif

      - name: Generate SBOM
        uses: anchore/sbom-action@v0
        with:
          image: ${{ steps.login-ecr.outputs.registry }}/erp-saas:${{ github.sha }}
          format: spdx-json
          output-file: sbom.spdx.json

      - name: Sign image with Cosign
        uses: sigstore/cosign-installer@v3

      - name: Push signed image
        env:
          ECR_REGISTRY: ${{ steps.login-ecr.outputs.registry }}
          IMAGE_TAG: ${{ github.sha }}
        run: |
          cosign sign --yes $ECR_REGISTRY/erp-saas:$IMAGE_TAG
          docker push $ECR_REGISTRY/erp-saas:$IMAGE_TAG

          # Also tag as latest for main branch
          if [ "${{ github.ref }}" == "refs/heads/main" ]; then
            docker tag $ECR_REGISTRY/erp-saas:$IMAGE_TAG $ECR_REGISTRY/erp-saas:latest
            docker push $ECR_REGISTRY/erp-saas:latest
          fi

  deploy:
    needs: build-and-scan
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4

      - name: Update Helm values
        run: |
          IMAGE_TAG=${{ github.sha }}
          sed -i "s|tag: .*|tag: $IMAGE_TAG|" deploy/charts/erp-saas/values.yaml

      - name: Commit changes
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add deploy/charts/erp-saas/values.yaml
          git commit -m "chore: update image to ${{ github.sha }}"
          git push
```

### 10.2 ArgoCD Application

```yaml
# deploy/argocd/application.yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: erp-saas
  namespace: argocd
spec:
  project: default

  source:
    repoURL: https://github.com/org/erp-saas-infra
    targetRevision: HEAD
    path: deploy/charts/erp-saas
    helm:
      valueFiles:
        - values.yaml
        - values-production.yaml

  destination:
    server: https://kubernetes.default.svc
    namespace: erp-saas

  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=true

  ignoreDifferences:
    - group: apps
      kind: Deployment
      jsonPointers:
        - /spec/replicas
```

---

## 11. Security

### 11.1 Security Architecture

```
+-----------------------------------------------------------------+
|                    Security Layers                               |
+-----------------------------------------------------------------+
|                                                                 |
|  Layer 1: Network Security                                      |
|  +-----------------------------------------------------------+ | |
|  | - VPC with private subnets                                | | |
|  | - Security groups (least privilege)                       | | |
|  | - Network policies (K8s)                                  | | |
|  | - WAF for public endpoints                                | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Layer 2: Authentication & Authorization                        |
|  +-----------------------------------------------------------+ | |
|  | - JWT tokens with short expiry                            | | |
|  | - OAuth2 / SSO integration                                | | |
|  | - RBAC in Kubernetes                                      | | |
|  | - Row-Level Security in database                          | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Layer 3: Data Protection                                       |
|  +-----------------------------------------------------------+ | |
|  | - Encryption at rest (KMS)                                | | |
|  | - Encryption in transit (TLS)                             | | |
|  | - Secrets management (Secrets Manager)                    | | |
|  | - Tenant data isolation                                   | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Layer 4: Application Security                                  |
|  +-----------------------------------------------------------+ | |
|  | - Input validation                                        | | |
|  | - SQL injection prevention (parameterized queries)        | | |
|  | - Rate limiting                                           | | |
|  | - Audit logging                                           | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
+-----------------------------------------------------------------+
```

### 11.2 JWT Authentication

```rust
// crates/auth-service/src/jwt.rs
use jsonwebtoken::{decode, encode, decode_header, DecodingKey, EncodingKey, Header, Algorithm, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,           // User ID
    pub jti: Uuid,          // JWT ID for revocation tracking
    pub tenant_id: Uuid,     // Tenant ID
    pub role: String,        // User role
    pub permissions: Vec<String>,
    pub exp: usize,          // Expiration
    pub iat: usize,          // Issued at
    pub iss: String,         // Issuer
    pub aud: String,         // Audience
}

/// Token revocation store (Redis-backed in production)
pub struct TokenRevocationStore {
    revoked_tokens: Arc<RwLock<HashSet<Uuid>>>,
}

impl TokenRevocationStore {
    pub fn new() -> Self {
        Self {
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn revoke(&self, jti: Uuid) {
        self.revoked_tokens.write().await.insert(jti);
    }

    pub async fn is_revoked(&self, jti: &Uuid) -> bool {
        self.revoked_tokens.read().await.contains(jti)
    }
}

/// RS256 JWT Service (Asymmetric Key - Recommended for Production)
pub struct JwtService {
    encoding_key: EncodingKey,      // RSA Private Key
    decoding_key: DecodingKey,      // RSA Public Key
    issuer: String,
    audience: String,
    access_token_expiry: i64,       // seconds
    refresh_token_expiry: i64,
    revocation_store: TokenRevocationStore,
}

impl JwtService {
    /// Initialize with RSA keys (RS256)
    /// Generate keys with: openssl genrsa -out private.pem 2048
    /// Extract public: openssl rsa -in private.pem -pubout -out public.pem
    pub fn new(private_key_pem: &str, public_key_pem: &str, issuer: String, audience: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
                .expect("Invalid RSA private key"),
            decoding_key: DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
                .expect("Invalid RSA public key"),
            issuer,
            audience,
            access_token_expiry: 900,      // 15 minutes
            refresh_token_expiry: 604800,  // 7 days
            revocation_store: TokenRevocationStore::new(),
        }
    }

    /// Generate access token with unique JTI for revocation support
    pub fn generate_access_token(&self, claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now().timestamp() as usize;
        let mut claims = claims.clone();
        claims.jti = Uuid::new_v4();  // Unique ID for revocation tracking
        claims.exp = now + self.access_token_expiry as usize;
        claims.iat = now;
        claims.iss = self.issuer.clone();
        claims.aud = self.audience.clone();

        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
    }

    /// Generate refresh token with rotation support
    pub fn generate_refresh_token(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(String, Uuid), jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now().timestamp() as usize;
        let jti = Uuid::new_v4();
        
        let claims = Claims {
            sub: user_id,
            jti,
            tenant_id,
            role: "refresh".to_string(),
            permissions: vec![],
            exp: now + self.refresh_token_expiry as usize,
            iat: now,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };

        let token = encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)?;
        Ok((token, jti))
    }

    /// Validate token with revocation check and algorithm whitelist
    pub async fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        // 1. Decode header first to check algorithm
        let header = decode_header(token)
            .map_err(|e| JwtError::InvalidToken(e.to_string()))?;

        // 2. Verify algorithm is RS256 (prevent algorithm confusion attack)
        if header.alg != Algorithm::RS256 {
            return Err(JwtError::InvalidAlgorithm);
        }

        // 3. Validate signature and claims
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&&self.audience);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| JwtError::InvalidToken(e.to_string()))?;

        // 4. Check if token is revoked
        if self.revocation_store.is_revoked(&token_data.claims.jti).await {
            return Err(JwtError::TokenRevoked);
        }

        Ok(token_data.claims)
    }

    /// Revoke a token (for logout, password change, etc.)
    pub async fn revoke_token(&self, jti: Uuid) {
        self.revocation_store.revoke(jti).await;
    }

    /// Refresh token rotation - revoke old, issue new
    pub async fn rotate_refresh_token(&self, old_jti: Uuid, claims: &Claims) -> Result<String, JwtError> {
        // Revoke old refresh token
        self.revoke_token(old_jti).await;
        
        // Generate new access token
        self.generate_access_token(claims)
            .map_err(|e| JwtError::TokenGeneration(e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("Invalid algorithm - only RS256 is allowed")]
    InvalidAlgorithm,
    #[error("Token has been revoked")]
    TokenRevoked,
    #[error("Failed to generate token: {0}")]
    TokenGeneration(String),
}
```

### 11.3 Rate Limiting

```rust
// crates/shared/src/middleware/rate_limit.rs
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// High-performance rate limiter using DashMap (lock-free concurrent map)
#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<DashMap<String, RateLimitEntry>>,
    limit: u32,
    window: Duration,
}

struct RateLimitEntry {
    count: u32,
    reset_at: Instant,
}

impl RateLimiter {
    pub fn new(limit: u32, window: Duration) -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
            limit,
            window,
        }
    }

    pub fn check(&self, key: &str) -> Result<(), StatusCode> {
        let now = Instant::now();

        // Atomic upsert with DashMap
        let entry = self.requests.entry(key.to_string()).or_insert_with(|| RateLimitEntry {
            count: 0,
            reset_at: now + self.window,
        });

        // Reset if window expired (atomic check-and-reset)
        if now >= entry.reset_at {
            entry.count = 0;
            entry.reset_at = now + self.window;
        }

        if entry.count >= self.limit {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        entry.count += 1;
        Ok(())
    }
}

/// Distributed rate limiter using Redis (for multi-instance deployments)
pub struct DistributedRateLimiter {
    redis: redis::Client,
    limit: u32,
    window: Duration,
}

impl DistributedRateLimiter {
    pub fn new(redis_url: &str, limit: u32, window: Duration) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { redis: client, limit, window })
    }

    pub async fn check(&self, key: &str) -> Result<(), StatusCode> {
        let mut conn = self.redis.get_async_connection().await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let redis_key = format!("ratelimit:{}", key);
        let window_secs = self.window.as_secs();

        // Sliding window rate limiting using Redis
        let count: u64 = redis::cmd("INCR")
            .arg(&redis_key)
            .query_async(&mut conn)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if count == 1 {
            // Set expiry on first request
            redis::cmd("EXPIRE")
                .arg(&redis_key)
                .arg(window_secs)
                .query_async(&mut conn)
                .await
                .ok();
        }

        if count > self.limit as u64 {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }

        Ok(())
    }
}

pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Use tenant_id + IP as key
    let key = format!(
        "{}:{}",
        req.headers()
            .get("x-tenant-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        req.headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
    );

    limiter.check(&key)?;
    Ok(next.run(req).await)
}
```

---

## 12. Monitoring & Observability

### 12.1 Monitoring Stack

```
+-----------------------------------------------------------------+
|                    Observability Stack                           |
+-----------------------------------------------------------------+
|                                                                 |
|  Metrics                                                        |
|  +----------------+  +----------------+  +----------------+      |
|  | Prometheus     |  | Grafana        |  | CloudWatch     |      |
|  | (Collection)   |  | (Visualization)|  | (AWS Native)   |      |
|  +----------------+  +----------------+  +----------------+      |
|                                                                 |
|  Logging                                                        |
|  +----------------+  +----------------+  +----------------+      |
|  | Fluent Bit     |  | Loki           |  | OpenSearch     |      |
|  | (Collection)   |  | (Storage)      |  | (AWS Native)   |      |
|  +----------------+  +----------------+  +----------------+      |
|                                                                 |
|  Tracing                                                        |
|  +----------------+  +----------------+                          |
|  | Jaeger         |  | AWS X-Ray      |                          |
|  | (Distributed)  |  | (AWS Native)   |                          |
|  +----------------+  +----------------+                          |
|                                                                 |
+-----------------------------------------------------------------+
```

### 12.2 Metrics Collection

### 12.3 Prometheus Alerts (Kubernetes and Application)

```yaml
# deploy/monitoring/prometheus-alerts.yaml
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: erp-kubernetes-alerts
  namespace: monitoring
spec:
  groups:
    - name: kubernetes.rules
      rules:
        - alert: KubeApiServerDown
          expr: absent(up{job="apiserver"} == 1)
          for: 1m
          labels:
            severity: critical
          annotations:
            summary: Kubernetes API server is down

        - alert: KubeControllerManagerDown
          expr: absent(up{job="kube-controller-manager"} == 1)
          for: 1m
          labels:
            severity: critical

        - alert: NodeNotReady
          expr: kube_node_status{condition="Ready"} == 0
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: Node {{ $labels.node }} is not ready

        - alert: PodCrashLooping
          expr: rate(kube_pod_container_status_waiting_reason{job="kube-system",namespace=~".+"}[5m]) > 0.1
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: Pod {{ $labels.pod }} in {{ $labels.namespace }} is crash loopinging

        - alert: HighMemoryUsage
          expr: (node_memory_Mbytes_total / node_memory_bytes_total) > 0.85
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: Node memory usage exceeds 85%

        - alert: PVCUsageCritical
          expr: kubelet_volume_capacity_bytes_total / kubelet_volume_capacity_bytes * 100 > 0.9
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: PVC {{ $labels.persistentvolumeclaim }} is almost full

        - alert: TenantRateLimitExceeded
          expr: |
            sum(rate(rate_limit_requests_total{status="429"}[5m])) by (tenant_id)
            > 10
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: Tenant {{ $labels.tenant_id }} rate limit exceeded
---
# SLO and Error Budget Alerting
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: slo-alerts
  namespace: monitoring
spec:
  groups:
    - name: slo.rules
      rules:
        - alert: ErrorBudgetBurn
          expr: |
            100 * (
              sum(rate(http_requests_total{status!~"5.."}[1h]) / sum(rate(http_requests_total[1h])))
              - (1 - 0.999) * 14.4  # Error budget burn rate: 2.3% / hour
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: Error budget burning fast - investigate immediately

        - alert: SLOAvailabilityBreached
          expr: |
            sum(rate(http_requests_total{status!~"5.."}[30d]) 
            / sum(rate(http_requests_total[30d]) < 0.999
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: Monthly availability SLO (99.9%) breached
---
# OpenTelemetry Collector Configuration
apiVersion: opentelemetry.io/v1beta1
kind: OpenTelemetryCollector
metadata:
  name: erp-otel-collector
  namespace: monitoring
spec:
  mode: deployment
  config: |
    receivers:
      otlp:
        protocols:
          grpc:
            endpoint: 0.0.0.0:4317
          http:
            endpoint: 0.0.0.0:4318

    processors:
      batch:
        timeout: 10s
      memory_limiter:
        check_interval: 1s
        limit_mib: 512MiB
      filter/span:
        spans_per_second: 100
        traces:
          status_code_filter:
            status_codes: [ERROR, UNSET]

    exporters:
      prometheus:
        endpoint: "0.0.0.0:8889"
        namespace: erp-saas
      jaeger:
        endpoint: jaeger-collector:14250
        tls:
          insecure: true
      logging:
        loglevel: info

    service:
      pipelines:
        traces:
          receivers: [otlp]
          processors: [memory_limiter, batch]
          exporters: [jaeger, prometheus]
        metrics:
          receivers: [otlp]
          processors: [memory_limiter, batch]
          exporters: [prometheus]
        logs:
          receivers: [otlp]
          processors: [memory_limiter]
          exporters: [logging]

---
# OpenTelemetry Instrumentation (Rust)
apiVersion: opentelemetry.io/v1alpha1
kind: OpenTelemetryInstrumentation
metadata:
  name: erp-services
  namespace: monitoring
spec:
  selector:
    matchLabels:
      app.kubernetes.io/part-of: erp-saas
  exporter:
    endpoint: http://erp-otel-collector:4317/v1/traces
    sampler: always_on
  env:
    - name: OTEL_SERVICE_NAME
      valueFrom: metadata.name
    - name: OTEL_K8S_NAMESPACE
      valueFrom: metadata.namespace
use prometheus::{
    register_counter_vec, register_histogram_vec, CounterVec, HistogramVec,
};

lazy_static! {
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "http_requests_total",
        "Total HTTP requests",
        &["tenant_id", "method", "path", "status"]
    ).unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration",
        &["tenant_id", "method", "path"],
        vec![0.1, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();

    pub static ref DB_QUERY_DURATION: HistogramVec = register_histogram_vec!(
        "db_query_duration_seconds",
        "Database query duration",
        &["tenant_id", "query_type"],
        vec![0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();

    pub static ref TENANT_ACTIVE_USERS: CounterVec = register_counter_vec!(
        "tenant_active_users_total",
        "Active users per tenant",
        &["tenant_id"]
    ).unwrap();
}

pub fn record_http_request(
    tenant_id: &str,
    method: &str,
    path: &str,
    status: u16,
    duration: std::time::Duration,
) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[tenant_id, method, path, &status.to_string()])
        .inc();

    HTTP_REQUEST_DURATION
        .with_label_values(&[tenant_id, method, path])
        .observe(duration.as_secs_f64());
}
```

### 12.3 Kubernetes Monitoring

```yaml
# deploy/monitoring/prometheus.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: erp-services
  namespace: monitoring
spec:
  selector:
    matchLabels:
      app.kubernetes.io/part-of: erp-saas
  namespaceSelector:
    any: true
  endpoints:
    - port: http
      path: /metrics
      interval: 30s
---
apiVersion: monitoring.coreos.com/v1
kind: PrometheusRule
metadata:
  name: erp-alerts
  namespace: monitoring
spec:
  groups:
    - name: erp-saas.rules
      rules:
        - alert: HighErrorRate
          expr: |
            sum(rate(http_requests_total{status=~"5.."}[5m])) by (namespace)
            / sum(rate(http_requests_total[5m])) by (namespace) > 0.05
          for: 5m
          labels:
            severity: critical
          annotations:
            summary: High error rate in {{ $labels.namespace }}

        - alert: TenantResourceQuotaExceeded
          expr: |
            kube_resourcequota{type="used", resource="requests.cpu"}
            / kube_resourcequota{type="hard", resource="requests.cpu"} > 0.9
          for: 5m
          labels:
            severity: warning
          annotations:
            summary: Tenant {{ $labels.namespace }} near resource quota

        - alert: DatabaseConnectionPoolExhausted
          expr: |
            db_connection_pool_used / db_connection_pool_size > 0.9
          for: 2m
          labels:
            severity: critical
          annotations:
            summary: Database connection pool nearly exhausted
```

---

## 13. Implementation Roadmap

### 13.1 Phase 1: Foundation (Weeks 1-4)

```
+-----------------------------------------------------------------+
|                    Phase 1: Foundation                           |
+-----------------------------------------------------------------+
|                                                                 |
|  Week 1-2: Infrastructure Setup                                 |
|  +-----------------------------------------------------------+ | |
|  | - AWS Account and VPC setup                               | | |
|  | - EKS cluster deployment                                  | | |
|  | - Aurora PostgreSQL setup                                 | | |
|  | - ElastiCache Redis setup                                 | | |
|  | - Local development environment (k3s)                     | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 3-4: Core Services                                        |
|  +-----------------------------------------------------------+ | |
|  | - API Gateway service                                     | | |
|  | - Auth service (JWT, OAuth2)                              | | |
|  | - Tenant service (provisioning)                           | | |
|  | - Feature flag service                                    | | |
|  | - Basic CI/CD pipeline                                    | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
+-----------------------------------------------------------------+
```

### 13.2 Phase 2: Core Modules (Weeks 5-10)

```
+-----------------------------------------------------------------+
|                    Phase 2: Core Modules                         |
+-----------------------------------------------------------------+
|                                                                 |
|  Week 5-6: Finance Module                                       |
|  +-----------------------------------------------------------+ | |
|  | - Chart of accounts                                       | | |
|  | - General ledger                                          | | |
|  | - Journal entries                                         | | |
|  | - Financial reports                                       | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 7-8: Inventory Module                                     |
|  +-----------------------------------------------------------+ | |
|  | - Products and SKUs                                       | | |
|  | - Stock management                                        | | |
|  | - Warehouse management                                    | | |
|  | - Stock movements                                         | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 9-10: Sales Module                                        |
|  +-----------------------------------------------------------+ | |
|  | - Customer management                                     | | |
|  | - Quotations                                              | | |
|  | - Sales orders                                            | | |
|  | - Invoicing                                               | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
+-----------------------------------------------------------------+
```

### 13.3 Phase 3: Scale & Polish (Weeks 11-16)

```
+-----------------------------------------------------------------+
|                    Phase 3: Scale & Polish                       |
+-----------------------------------------------------------------+
|                                                                 |
|  Week 11-12: Additional Modules                                 |
|  +-----------------------------------------------------------+ | |
|  | - HR module                                               | | |
|  | - Procurement module                                      | | |
|  | - Basic reporting                                         | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 13-14: Advanced Features                                  |
|  +-----------------------------------------------------------+ | |
|  | - Multi-warehouse support                                 | | |
|  | - Advanced budgeting                                      | | |
|  | - Dashboard and analytics                                 | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 15-16: Production Readiness                               |
|  +-----------------------------------------------------------+ | |
|  | - Full observability stack                                | | |
|  | - Security hardening                                      | | |
|  | - Performance testing                                     | | |
|  | - Documentation                                           | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
+-----------------------------------------------------------------+
```

### 13.4 Phase 4: Enterprise Ready (Weeks 17-24)

```
+-----------------------------------------------------------------+
|                    Phase 4: Enterprise Ready                     |
+-----------------------------------------------------------------+
|                                                                 |
|  Week 17-18: Enterprise Features                                |
|  +-----------------------------------------------------------+ | |
|  | - SSO / SAML integration                                  | | |
|  | - Production module                                       | | |
|  | - WMS module                                              | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 19-20: Advanced Security                                  |
|  +-----------------------------------------------------------+ | |
|  | - Audit logging                                           | | |
|  | - Data encryption per tenant                              | | |
|  | - Compliance features (SOC2, GDPR)                        | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 21-22: Premium Features                                   |
|  +-----------------------------------------------------------+ | |
|  | - AI forecasting (beta)                                   | | |
|  | - Advanced analytics                                      | | |
|  | - Custom integrations                                     | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
|  Week 23-24: Launch Preparation                                 |
|  +-----------------------------------------------------------+ | |
|  | - Load testing                                            | | |
|  | - Security audit                                          | | |
|  | - Documentation complete                                  | | |
|  | - Go-live preparation                                     | | |
|  +-----------------------------------------------------------+ | |
|                                                                 |
+-----------------------------------------------------------------+
```

### 13.5 Resource Requirements

| Phase | Developers | Duration | Key Deliverables |
|-------|------------|----------|------------------|
| Phase 1 | 2-3 | 4 weeks | Infrastructure, Auth, Tenant mgmt |
| Phase 2 | 3-4 | 6 weeks | Finance, Inventory, Sales modules |
| Phase 3 | 4-5 | 6 weeks | HR, Procurement, Reporting |
| Phase 4 | 4-5 | 8 weeks | Enterprise features, Security |

---

## Appendix A: Quick Reference

### Common Commands

```bash
# Local development
make dev-up          # Start local infrastructure
make dev-down        # Stop local infrastructure
make db-migrate      # Run database migrations
make test            # Run tests

# Kubernetes
kubectl apply -k deploy/overlays/local    # Deploy to local k8s
kubectl logs -f deployment/api-gateway    # View logs
kubectl get pods -A                       # List all pods

# Docker
docker compose up -d                      # Start services
docker compose logs -f                    # View logs
docker build -t erp-saas:latest .         # Build image
```

### Environment Variables

```bash
# Database
DATABASE_URL=postgresql://db_user:dev_password@localhost:5432/erp

# Redis
REDIS_URL=redis://localhost:6379

# JWT
JWT_SECRET=your-secret-key
JWT_ISSUER=erp-saas

# AWS (local)
AWS_ACCESS_KEY_ID=local_dev_user
AWS_SECRET_ACCESS_KEY=local_dev_secret
S3_ENDPOINT=http://localhost:9000
LOCALSTACK_ENDPOINT=http://localhost:4566

# Environment
ENVIRONMENT=local
RUST_LOG=debug
```

---

**Document Version:** 1.0.0
**Last Updated:** 2026-03-03
**Maintainers:** ERP SaaS Team
