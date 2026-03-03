//! Authentication utilities

use jsonwebtoken::{decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// JWT Claims structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (User ID)
    pub sub: Uuid,
    /// JWT ID for revocation tracking
    pub jti: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// User role
    pub role: String,
    /// Permissions list
    pub permissions: Vec<String>,
    /// Expiration timestamp
    pub exp: usize,
    /// Issued at timestamp
    pub iat: usize,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
}

/// In-memory token revocation store
/// In production, this should be backed by Redis
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
    
    pub async fn revoke_all_for_user(&self, user_id: Uuid) {
        // In production, this would query Redis to find all tokens for user
        // For now, we just log this action
        tracing::warn!("Token revocation requested for user {}", user_id);
    }
}

impl Default for TokenRevocationStore {
    fn default() -> Self {
        Self::new()
    }
}

/// JWT Service using RS256 (RSA)
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    issuer: String,
    audience: String,
    access_token_expiry: i64,
    refresh_token_expiry: i64,
    revocation_store: TokenRevocationStore,
}

impl JwtService {
    /// Create a new JWT service from RSA key files
    pub fn from_files(
        private_key_path: &str,
        public_key_path: &str,
        issuer: String,
        audience: String,
        access_token_expiry: i64,
        refresh_token_expiry: i64,
    ) -> Result<Self, crate::Error> {
        let private_key = std::fs::read(private_key_path)
            .map_err(|e| crate::Error::Config(format!("Failed to read private key: {}", e)))?;
        let public_key = std::fs::read(public_key_path)
            .map_err(|e| crate::Error::Config(format!("Failed to read public key: {}", e)))?;
        
        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(&private_key)
                .map_err(|e| crate::Error::Config(format!("Invalid private key: {}", e)))?,
            decoding_key: DecodingKey::from_rsa_pem(&public_key)
                .map_err(|e| crate::Error::Config(format!("Invalid public key: {}", e)))?,
            issuer,
            audience,
            access_token_expiry,
            refresh_token_expiry,
            revocation_store: TokenRevocationStore::new(),
        })
    }
    
    /// Create from raw PEM data
    pub fn new(
        private_key_pem: &[u8],
        public_key_pem: &[u8],
        issuer: String,
        audience: String,
    ) -> Result<Self, crate::Error> {
        Ok(Self {
            encoding_key: EncodingKey::from_rsa_pem(private_key_pem)
                .map_err(|e| crate::Error::Config(format!("Invalid private key: {}", e)))?,
            decoding_key: DecodingKey::from_rsa_pem(public_key_pem)
                .map_err(|e| crate::Error::Config(format!("Invalid public key: {}", e)))?,
            issuer,
            audience,
            access_token_expiry: 900,      // 15 minutes
            refresh_token_expiry: 604800,  // 7 days
            revocation_store: TokenRevocationStore::new(),
        })
    }

    /// Generate an access token
    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        role: String,
        permissions: Vec<String>,
    ) -> Result<String, crate::Error> {
        let now = chrono::Utc::now().timestamp() as usize;
        let jti = Uuid::new_v4();

        let claims = Claims {
            sub: user_id,
            jti,
            tenant_id,
            role,
            permissions,
            exp: now + self.access_token_expiry as usize,
            iat: now,
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
        };

        encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| crate::Error::Auth(format!("Token generation failed: {}", e)))
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(&self, user_id: Uuid, tenant_id: Uuid) -> Result<(String, Uuid), crate::Error> {
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

        let token = encode(&Header::new(Algorithm::RS256), &claims, &self.encoding_key)
            .map_err(|e| crate::Error::Auth(format!("Token generation failed: {}", e)))?;
        
        Ok((token, jti))
    }

    /// Validate a token and return claims
    pub async fn validate_token(&self, token: &str) -> Result<Claims, crate::Error> {
        // 1. Decode header to check algorithm
        let header = decode_header(token)
            .map_err(|e| crate::Error::Auth(format!("Invalid token header: {}", e)))?;

        // 2. Verify algorithm is RS256 (prevent algorithm confusion)
        if header.alg != Algorithm::RS256 {
            return Err(crate::Error::Auth("Invalid algorithm - only RS256 allowed".to_string()));
        }

        // 3. Validate signature and claims
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(crate::Error::from)?;

        // 4. Check if token is revoked
        if self.revocation_store.is_revoked(&token_data.claims.jti).await {
            return Err(crate::Error::TokenRevoked);
        }

        Ok(token_data.claims)
    }

    /// Revoke a token
    pub async fn revoke_token(&self, jti: Uuid) {
        self.revocation_store.revoke(jti).await;
    }
}

/// Password hashing utilities using Argon2
pub struct PasswordHasher {
    hasher: argon2::Argon2<'static>,
}

impl PasswordHasher {
    pub fn new() -> Self {
        Self {
            hasher: argon2::Argon2::default(),
        }
    }

    /// Hash a password
    pub fn hash(&self, password: &str) -> Result<String, crate::Error> {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        
        let salt = SaltString::generate(&mut OsRng);
        self.hasher
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| crate::Error::Auth(format!("Password hashing failed: {}", e)))
    }

    /// Verify a password against a hash
    pub fn verify(&self, password: &str, hash: &str) -> Result<bool, crate::Error> {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};
        
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| crate::Error::Auth(format!("Invalid hash format: {}", e)))?;
        
        self.hasher
            .verify_password(password.as_bytes(), &parsed_hash)
            .map(|_| true)
            .or_else(|e| match e {
                argon2::password_hash::Error::Password => Ok(false),
                _ => Err(crate::Error::Auth(format!("Password verification failed: {}", e))),
            })
    }
}

impl Default for PasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let hasher = PasswordHasher::new();
        let password = "test_password_123";
        
        let hash = hasher.hash(password).expect("Failed to hash password");
        assert!(hasher.verify(password, &hash).expect("Failed to verify password"));
        assert!(!hasher.verify("wrong_password", &hash).expect("Failed to verify wrong password"));
    }
}
