//! Test fixtures and data factories for auth-service tests
//!
//! Provides reusable test data using the `fake` crate for random generation

#![allow(dead_code)]

use chrono::Utc;
use fake::{Fake, Faker};
use shared::tenant::{IsolationLevel, Plan, Tenant};
use uuid::Uuid;

/// Test fixtures factory for generating test data
pub struct TestFixtures;

impl TestFixtures {
    /// Create a mock tenant with default values
    pub fn mock_tenant() -> Tenant {
        Tenant {
            id: Uuid::new_v4(),
            name: Faker.fake(),
            slug: Faker.fake(),
            isolation_level: IsolationLevel::Pool,
            plan: Plan::Starter,
            is_active: true,
            schema_name: None,
            database_url: None,
        }
    }

    /// Create a mock tenant with specific isolation level
    pub fn mock_tenant_with_isolation(isolation: IsolationLevel) -> Tenant {
        let mut tenant = Self::mock_tenant();
        tenant.isolation_level = isolation;
        tenant
    }

    /// Create a mock tenant with specific plan
    pub fn mock_tenant_with_plan(plan: Plan) -> Tenant {
        let mut tenant = Self::mock_tenant();
        tenant.plan = plan;
        tenant
    }

    /// Create a valid test email
    pub fn test_email() -> String {
        format!("test_{}@example.com", Uuid::new_v4())
    }

    /// Create a valid test password
    pub fn test_password() -> String {
        "TestPassword123!@#".to_string()
    }

    /// Create a weak password for testing validation
    pub fn weak_password() -> String {
        "123".to_string()
    }

    /// Create a test JWT claims payload
    pub fn mock_jwt_claims() -> shared::auth::Claims {
        shared::auth::Claims {
            sub: Uuid::new_v4(),
            jti: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role: "user".to_string(),
            permissions: vec!["users:read".to_string()],
            exp: (Utc::now().timestamp() + 900) as usize,
            iat: Utc::now().timestamp() as usize,
            iss: "erp-saas".to_string(),
            aud: "erp-saas-api".to_string(),
        }
    }

    /// Create test user data for registration
    pub fn registration_data() -> RegistrationData {
        RegistrationData {
            email: Self::test_email(),
            password: Self::test_password(),
            name: Faker.fake(),
            tenant_slug: Some(Faker.fake()),
        }
    }
}

/// Registration test data
#[derive(Debug, Clone)]
pub struct RegistrationData {
    pub email: String,
    pub password: String,
    pub name: String,
    pub tenant_slug: Option<String>,
}

/// Login test data
#[derive(Debug, Clone)]
pub struct LoginTestData {
    pub email: String,
    pub password: String,
    pub tenant_slug: Option<String>,
}

impl LoginTestData {
    pub fn valid() -> Self {
        Self {
            email: TestFixtures::test_email(),
            password: TestFixtures::test_password(),
            tenant_slug: None,
        }
    }

    pub fn invalid_password() -> Self {
        Self {
            email: TestFixtures::test_email(),
            password: "wrong_password".to_string(),
            tenant_slug: None,
        }
    }

    pub fn nonexistent_user() -> Self {
        Self {
            email: "nonexistent@example.com".to_string(),
            password: TestFixtures::test_password(),
            tenant_slug: None,
        }
    }
}
