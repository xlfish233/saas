# 安全设计

## 安全架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      安全边界                                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Internet                                                       │
│      │                                                          │
│      ▼                                                          │
│  ┌─────────────┐                                                │
│  │  AWS WAF    │  ← L7 防护 (SQLi, XSS, Rate Limit)            │
│  └──────┬──────┘                                                │
│         │                                                       │
│  ┌──────▼──────┐                                                │
│  │  ALB + TLS  │  ← TLS 终止, 证书管理                          │
│  └──────┬──────┘                                                │
│         │                                                       │
│  ┌──────▼──────┐                                                │
│  │   Istio     │  ← mTLS, 服务网格                              │
│  │  Gateway    │                                                │
│  └──────┬──────┘                                                │
│         │                                                       │
│  ┌──────▼──────┐                                                │
│  │ API Gateway │  ← 认证, 授权, 租户隔离                        │
│  │ (JWT RS256) │                                                │
│  └──────┬──────┘                                                │
│         │                                                       │
│  ┌──────▼──────┐                                                │
│  │  Services   │  ← RBAC, 数据隔离, 审计日志                    │
│  └─────────────┘                                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 认证流程

### JWT RS256 认证

```rust
// 使用非对称密钥 (RS256) 替代共享密钥 (HS256)
pub struct JwtService {
    encoding_key: EncodingKey,  // RSA 私钥
    decoding_key: DecodingKey,  // RSA 公钥
}

// Token 结构
pub struct Claims {
    sub: Uuid,           // 用户 ID
    jti: Uuid,          // JWT ID (用于撤销)
    tenant_id: Uuid,    // 租户 ID
    role: String,       // 角色
    permissions: Vec<String>,
    exp: usize,         // 过期时间
    iat: usize,         // 签发时间
    iss: String,        // 签发者
    aud: String,        // 受众
}
```

### 认证流程

```
1. 用户登录 → 验证凭证
2. 生成 Access Token (15分钟) + Refresh Token (7天)
3. Access Token 包含: user_id, tenant_id, role, permissions
4. 每次请求: 验证签名 → 检查过期 → 检查撤销 → 提取 Claims
5. Token 撤销: Redis 存储已撤销 JTI
```

## 授权 (RBAC)

```rust
// 角色定义
pub enum Role {
    SuperAdmin,      // 平台管理员
    TenantAdmin,     // 租户管理员
    Manager,         // 管理者
    Accountant,      // 会计
    SalesPerson,     // 销售人员
    Viewer,          // 只读用户
}

// 权限检查
pub fn check_permission(
    user_permissions: &[String],
    required: &str,
) -> bool {
    user_permissions.iter().any(|p| {
        p == required || 
        p.ends_with(":*") && required.starts_with(&p[..p.len()-1])
    })
}
```

### 角色权限矩阵

| 权限 | SuperAdmin | TenantAdmin | Manager | Accountant | Sales | Viewer |
|-----|:----------:|:-----------:|:-------:|:----------:|:-----:|:------:|
| users:* | ✓ | ✓ | - | - | - | - |
| finance:read | ✓ | ✓ | ✓ | ✓ | - | ✓ |
| finance:write | ✓ | ✓ | ✓ | ✓ | - | - |
| sales:* | ✓ | ✓ | ✓ | - | ✓ | - |
| inventory:read | ✓ | ✓ | ✓ | - | ✓ | ✓ |
| inventory:write | ✓ | ✓ | ✓ | - | - | - |
| settings:* | ✓ | ✓ | - | - | - | - |
| tenants:* | ✓ | - | - | - | - | - |

## 数据安全

### 传输加密

- TLS 1.3 用于所有外部通信
- mTLS 用于服务间通信 (Istio)
- RDS/Aurora 强制 SSL 连接

### 静态加密

| 资源 | 加密方式 | 密钥管理 |
|-----|---------|---------|
| Aurora | AES-256 | KMS |
| S3 | AES-256 / KMS | KMS |
| Secrets | Secrets Manager | KMS |
| EBS | AES-256 | KMS |

### 字段级加密 (敏感数据)

```rust
// 敏感字段加密
pub struct EncryptedField(String);

impl EncryptedField {
    pub fn encrypt(plaintext: &str, key: &KmsKey) -> Result<Self, Error> {
        let encrypted = key.encrypt(plaintext.as_bytes())?;
        Ok(Self(base64::encode(encrypted)))
    }
    
    pub fn decrypt(&self, key: &KmsKey) -> Result<String, Error> {
        let bytes = base64::decode(&self.0)?;
        let decrypted = key.decrypt(&bytes)?;
        Ok(String::from_utf8(decrypted)?)
    }
}

// 用于 PII 字段
pub struct Customer {
    pub id: Uuid,
    pub name: String,
    pub email: EncryptedField,      // 加密
    pub phone: EncryptedField,      // 加密
    pub tax_id: EncryptedField,     // 加密
}
```

## 网络安全

### Network Policies

```yaml
# 默认拒绝所有入站
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: default-deny-ingress
spec:
  podSelector: {}
  policyTypes:
    - Ingress
---
# 仅允许来自 Istio Gateway
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: allow-from-gateway
spec:
  podSelector:
    matchLabels:
      app: api-gateway
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: istio-system
```

### EKS 端点隔离

```hcl
# 禁用集群公共端点
cluster_endpoint_public_access  = false
cluster_endpoint_private_access = true

# 启用控制平面日志
enabled_cluster_log_types = ["api", "audit", "authenticator"]
```

## 审计日志

```rust
// 审计事件
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub action: String,           // "user.login", "order.create"
    pub resource_type: String,    // "user", "order", "invoice"
    pub resource_id: Uuid,
    pub changes: JsonValue,       // 变更前后数据
    pub ip_address: String,
    pub user_agent: String,
    pub status: AuditStatus,      // Success, Failure, Denied
}

// 不可变审计日志存储
// - 写入 S3 (append-only bucket)
// - 保留 7 年
// - 启用 Object Lock (WORM)
```

## 安全扫描

### CI/CD 集成

```yaml
# 安全扫描流水线
jobs:
  sast:
    runs-on: ubuntu-latest
    steps:
      - name: Semgrep SAST
        uses: returntocorp/semgrep-action@v1
        
  secret-scan:
    runs-on: ubuntu-latest
    steps:
      - name: Gitleaks
        uses: gitleaks/gitleaks-action@v2
        
  container-scan:
    runs-on: ubuntu-latest
    steps:
      - name: Trivy
        uses: aquasecurity/trivy-action@master
```

## 合规性

| 标准 | 状态 | 关键要求 |
|-----|-----|---------|
| SOC 2 Type II | ✓ | 访问控制、审计日志、加密 |
| GDPR | ✓ | 数据主体权利、遗忘权 |
| PCI DSS | ✓ | 支付数据处理 |
| HIPAA | 可选 | 医疗数据保护 |

## 安全最佳实践

1. **最小权限原则**: 默认拒绝，按需授权
2. **零信任**: 每次请求验证身份和权限
3. **防御深度**: 多层安全控制
4. **安全开发**: 代码审查、安全扫描
5. **事件响应**: 安全事件监控和响应流程
6. **定期审计**: 定期安全评估和渗透测试
