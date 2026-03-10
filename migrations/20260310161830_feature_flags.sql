-- Feature Flags Table
-- Creates tables for feature flag management

-- Feature flags table
CREATE TABLE features (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key VARCHAR(100) NOT NULL UNIQUE,  -- e.g., "finance", "inventory.multi_warehouse"
    name VARCHAR(255) NOT NULL,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT true,
    required_tier VARCHAR(50),  -- starter, pro, enterprise
    rollout_percentage INTEGER DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Tenant feature assignments
CREATE TABLE tenant_features (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    feature_id UUID NOT NULL REFERENCES features(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, feature_id)
);

-- Index for tenant lookups
CREATE INDEX idx_tenant_features_tenant ON tenant_features(tenant_id);
CREATE INDEX idx_tenant_features_feature ON tenant_features(feature_id);
CREATE INDEX idx_features_key ON features(key);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_features_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for features table
CREATE TRIGGER trigger_features_updated_at
    BEFORE UPDATE ON features
    FOR EACH ROW
    EXECUTE FUNCTION update_features_updated_at();

-- Seed data for default features
INSERT INTO features (key, name, description, enabled, required_tier, rollout_percentage) VALUES
    ('finance', 'Finance Module', 'Core financial management including accounting and reporting', true, 'starter', 100),
    ('inventory', 'Inventory Module', 'Basic inventory tracking and management', true, 'starter', 100),
    ('sales', 'Sales Module', 'Sales order management and customer relationships', true, 'starter', 100),
    ('inventory.multi_warehouse', 'Multi-Warehouse Inventory', 'Manage inventory across multiple warehouses', true, 'pro', 100),
    ('hr', 'Human Resources Module', 'Employee management, payroll, and benefits', true, 'pro', 100),
    ('production', 'Production Module', 'Manufacturing and production planning', true, 'enterprise', 100),
    ('ai_forecasting', 'AI Forecasting', 'AI-powered demand forecasting and analytics', true, 'enterprise', 50);
