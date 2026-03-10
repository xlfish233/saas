-- Tenant Usage Table
-- Tracks resource usage per tenant for quota enforcement

-- Tenant usage tracking table
CREATE TABLE tenant_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_count INTEGER NOT NULL DEFAULT 0,
    storage_used_bytes BIGINT NOT NULL DEFAULT 0,
    api_calls_count BIGINT NOT NULL DEFAULT 0,
    storage_files_count BIGINT NOT NULL DEFAULT 0,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, period_start)
);

-- Indexes for efficient lookups
CREATE INDEX idx_tenant_usage_tenant ON tenant_usage(tenant_id);
CREATE INDEX idx_tenant_usage_period ON tenant_usage(period_start, period_end);
CREATE INDEX idx_tenant_usage_tenant_period ON tenant_usage(tenant_id, period_start, period_end);

-- Function to get or create current usage record for a tenant
CREATE OR REPLACE FUNCTION get_or_create_usage(tenant_uuid UUID, period_start_ts TIMESTAMPTZ, period_end_ts TIMESTAMPTZ)
RETURNS UUID AS $$
DECLARE
    usage_id UUID;
BEGIN
    -- Try to get existing record
    SELECT id INTO usage_id FROM tenant_usage
    WHERE tenant_id = tenant_uuid AND period_start = period_start_ts;

    -- If not exists, create one
    IF usage_id IS NULL THEN
        INSERT INTO tenant_usage (tenant_id, period_start, period_end)
        VALUES (tenant_uuid, period_start_ts, period_end_ts)
        RETURNING id INTO usage_id;
    END IF;

    RETURN usage_id;
END;
$$ LANGUAGE plpgsql;

-- Trigger function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_tenant_usage_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for tenant_usage table
CREATE TRIGGER trigger_tenant_usage_updated_at
    BEFORE UPDATE ON tenant_usage
    FOR EACH ROW
    EXECUTE FUNCTION update_tenant_usage_updated_at();

-- Function to increment API calls count atomically
CREATE OR REPLACE FUNCTION increment_api_calls(tenant_uuid UUID, period_start_ts TIMESTAMPTZ, period_end_ts TIMESTAMPTZ)
RETURNS void AS $$
BEGIN
    INSERT INTO tenant_usage (tenant_id, period_start, period_end, api_calls_count)
    VALUES (tenant_uuid, period_start_ts, period_end_ts, 1)
    ON CONFLICT (tenant_id, period_start)
    DO UPDATE SET api_calls_count = tenant_usage.api_calls_count + 1;
END;
$$ LANGUAGE plpgsql;

-- Function to increment user count
CREATE OR REPLACE FUNCTION increment_user_count(tenant_uuid UUID, period_start_ts TIMESTAMPTZ, period_end_ts TIMESTAMPTZ)
RETURNS void AS $$
BEGIN
    INSERT INTO tenant_usage (tenant_id, period_start, period_end, user_count)
    VALUES (tenant_uuid, period_start_ts, period_end_ts, 1)
    ON CONFLICT (tenant_id, period_start)
    DO UPDATE SET user_count = tenant_usage.user_count + 1;
END;
$$ LANGUAGE plpgsql;

-- Function to decrement user count
CREATE OR REPLACE FUNCTION decrement_user_count(tenant_uuid UUID, period_start_ts TIMESTAMPTZ, period_end_ts TIMESTAMPTZ)
RETURNS void AS $$
BEGIN
    INSERT INTO tenant_usage (tenant_id, period_start, period_end, user_count)
    VALUES (tenant_uuid, period_start_ts, period_end_ts, 0)
    ON CONFLICT (tenant_id, period_start)
    DO UPDATE SET user_count = GREATEST(tenant_usage.user_count - 1, 0);
END;
$$ LANGUAGE plpgsql;

-- Function to update storage usage
CREATE OR REPLACE FUNCTION update_storage_usage(tenant_uuid UUID, bytes_delta BIGINT, files_delta BIGINT, period_start_ts TIMESTAMPTZ, period_end_ts TIMESTAMPTZ)
RETURNS void AS $$
BEGIN
    INSERT INTO tenant_usage (tenant_id, period_start, period_end, storage_used_bytes, storage_files_count)
    VALUES (tenant_uuid, period_start_ts, period_end_ts, GREATEST(bytes_delta, 0), GREATEST(files_delta, 0))
    ON CONFLICT (tenant_id, period_start)
    DO UPDATE SET
        storage_used_bytes = GREATEST(tenant_usage.storage_used_bytes + bytes_delta, 0),
        storage_files_count = GREATEST(tenant_usage.storage_files_count + files_delta, 0);
END;
$$ LANGUAGE plpgsql;
