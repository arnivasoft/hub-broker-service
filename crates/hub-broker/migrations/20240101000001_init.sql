-- Initial schema for Hub-Broker Service
-- Multi-tenant architecture with strict isolation

-- Tenants table (main isolation boundary)
CREATE TABLE IF NOT EXISTS tenants (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    company_name VARCHAR(255) NOT NULL,
    contact_email VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    max_branches INTEGER NOT NULL DEFAULT 10,
    max_connections_per_branch INTEGER NOT NULL DEFAULT 5,
    rate_limit_per_sec INTEGER NOT NULL DEFAULT 100,
    database_schema VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tenants_status ON tenants(status);
CREATE INDEX idx_tenants_created_at ON tenants(created_at);

-- Branches table (belongs to tenants)
CREATE TABLE IF NOT EXISTS branches (
    id VARCHAR(255) NOT NULL,
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    location VARCHAR(255),
    status VARCHAR(50) NOT NULL DEFAULT 'offline',
    api_key_hash TEXT NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, id)
);

CREATE INDEX idx_branches_tenant_id ON branches(tenant_id);
CREATE INDEX idx_branches_status ON branches(status);
CREATE INDEX idx_branches_updated_at ON branches(updated_at);

-- Sync transactions table (audit trail)
CREATE TABLE IF NOT EXISTS sync_transactions (
    id VARCHAR(255) PRIMARY KEY,
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    source_branch_id VARCHAR(255) NOT NULL,
    target_branch_id VARCHAR(255),
    transaction_type VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    changes_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP WITH TIME ZONE,
    FOREIGN KEY (tenant_id, source_branch_id) REFERENCES branches(tenant_id, id),
    FOREIGN KEY (tenant_id, target_branch_id) REFERENCES branches(tenant_id, id)
);

CREATE INDEX idx_sync_transactions_tenant_id ON sync_transactions(tenant_id);
CREATE INDEX idx_sync_transactions_status ON sync_transactions(status);
CREATE INDEX idx_sync_transactions_started_at ON sync_transactions(started_at);

-- Offline messages table (message queue for offline branches)
CREATE TABLE IF NOT EXISTS offline_messages (
    id VARCHAR(255) PRIMARY KEY,
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_branch_id VARCHAR(255) NOT NULL,
    message_payload JSONB NOT NULL,
    priority INTEGER NOT NULL DEFAULT 5,
    ttl TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    delivered_at TIMESTAMP WITH TIME ZONE,
    FOREIGN KEY (tenant_id, target_branch_id) REFERENCES branches(tenant_id, id)
);

CREATE INDEX idx_offline_messages_tenant_branch ON offline_messages(tenant_id, target_branch_id);
CREATE INDEX idx_offline_messages_delivered ON offline_messages(delivered_at) WHERE delivered_at IS NULL;
CREATE INDEX idx_offline_messages_ttl ON offline_messages(ttl);

-- Conflict resolutions table
CREATE TABLE IF NOT EXISTS conflict_resolutions (
    id VARCHAR(255) PRIMARY KEY,
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    table_name VARCHAR(255) NOT NULL,
    primary_key JSONB NOT NULL,
    branch_a_id VARCHAR(255) NOT NULL,
    branch_b_id VARCHAR(255) NOT NULL,
    branch_a_change JSONB NOT NULL,
    branch_b_change JSONB NOT NULL,
    resolution_strategy VARCHAR(50) NOT NULL,
    winning_branch_id VARCHAR(255),
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    FOREIGN KEY (tenant_id, branch_a_id) REFERENCES branches(tenant_id, id),
    FOREIGN KEY (tenant_id, branch_b_id) REFERENCES branches(tenant_id, id)
);

CREATE INDEX idx_conflict_resolutions_tenant ON conflict_resolutions(tenant_id);
CREATE INDEX idx_conflict_resolutions_resolved ON conflict_resolutions(resolved_at) WHERE resolved_at IS NULL;

-- Audit log table
CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    tenant_id VARCHAR(255) NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    branch_id VARCHAR(255),
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB,
    ip_address INET,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_tenant_id ON audit_log(tenant_id);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX idx_audit_log_event_type ON audit_log(event_type);

-- Trigger to auto-update updated_at columns
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_tenants_updated_at BEFORE UPDATE ON tenants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_branches_updated_at BEFORE UPDATE ON branches
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Sample data for testing (remove in production)
INSERT INTO tenants (id, name, company_name, contact_email, database_schema)
VALUES
    ('tenant_demo', 'Demo Tenant', 'Demo Company', 'demo@example.com', 'tenant_demo_schema'),
    ('tenant_test', 'Test Tenant', 'Test Company', 'test@example.com', 'tenant_test_schema')
ON CONFLICT (id) DO NOTHING;

-- Create schemas for sample tenants
CREATE SCHEMA IF NOT EXISTS tenant_demo_schema;
CREATE SCHEMA IF NOT EXISTS tenant_test_schema;
