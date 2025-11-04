use common::{BranchId, TenantId, Tenant, BranchInfo, Result, Error};
use sqlx::{PgPool, postgres::PgPoolOptions};
use redis::aio::ConnectionManager as RedisConnectionManager;
use std::time::Duration;
use tracing::info;

/// Storage layer handles all persistence
/// CRITICAL: Implements tenant isolation at database level
#[derive(Clone)]
pub struct Storage {
    pg_pool: PgPool,
    redis: RedisConnectionManager,
}

impl Storage {
    pub async fn new(config: &crate::config::Config) -> Result<Self> {
        // PostgreSQL connection pool
        let pg_pool = PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .min_connections(config.database.min_connections)
            .acquire_timeout(Duration::from_secs(config.database.connect_timeout_secs))
            .connect(&config.database.url)
            .await
            .map_err(|e| Error::DatabaseError(e))?;

        info!("PostgreSQL pool created");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pg_pool)
            .await
            .map_err(|e| Error::DatabaseError(e.into()))?;

        info!("Database migrations applied");

        // Redis connection
        let redis_client = redis::Client::open(config.redis.url.as_str())
            .map_err(|e| Error::RedisError(e.to_string()))?;

        let redis = redis::aio::ConnectionManager::new(redis_client)
            .await
            .map_err(|e| Error::RedisError(e.to_string()))?;

        info!("Redis connection established");

        Ok(Self { pg_pool, redis })
    }

    /// Get tenant by ID
    pub async fn get_tenant(&self, tenant_id: &TenantId) -> Result<Tenant> {
        let row = sqlx::query_as::<_, TenantRow>(
            "SELECT * FROM tenants WHERE id = $1"
        )
        .bind(tenant_id.as_str())
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        Ok(row.into())
    }

    /// Get branch with tenant validation
    /// CRITICAL: Always validates tenant ownership
    pub async fn get_branch(&self, tenant_id: &TenantId, branch_id: &BranchId) -> Result<BranchRow> {
        let row = sqlx::query_as::<_, BranchRow>(
            "SELECT * FROM branches WHERE id = $1 AND tenant_id = $2"
        )
        .bind(branch_id.as_str())
        .bind(tenant_id.as_str())
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        Ok(row)
    }

    /// Get API key hash for branch
    pub async fn get_api_key_hash(&self, tenant_id: &TenantId, branch_id: &BranchId) -> Result<String> {
        let row: (String,) = sqlx::query_as(
            "SELECT api_key_hash FROM branches WHERE id = $1 AND tenant_id = $2"
        )
        .bind(branch_id.as_str())
        .bind(tenant_id.as_str())
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        Ok(row.0)
    }

    /// Get tenant for a branch
    pub async fn get_tenant_for_branch(&self, branch_id: &BranchId) -> Result<TenantId> {
        let row: (String,) = sqlx::query_as(
            "SELECT tenant_id FROM branches WHERE id = $1"
        )
        .bind(branch_id.as_str())
        .fetch_one(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        Ok(TenantId::new(row.0))
    }

    /// List all branches for a tenant
    /// CRITICAL: Only returns branches belonging to specified tenant
    pub async fn list_branches_for_tenant(&self, tenant_id: &TenantId) -> Result<Vec<BranchInfo>> {
        let rows = sqlx::query_as::<_, BranchRow>(
            "SELECT * FROM branches WHERE tenant_id = $1 AND status = 'online'"
        )
        .bind(tenant_id.as_str())
        .fetch_all(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    /// Create new tenant (admin operation)
    pub async fn create_tenant(&self, tenant: &Tenant) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tenants (id, name, company_name, contact_email, status, max_branches,
                                max_connections_per_branch, rate_limit_per_sec, database_schema)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(tenant.id.as_str())
        .bind(&tenant.name)
        .bind(&tenant.company_name)
        .bind(&tenant.contact_email)
        .bind(format!("{:?}", tenant.status))
        .bind(tenant.max_branches as i32)
        .bind(tenant.max_connections_per_branch as i32)
        .bind(tenant.rate_limit_per_sec as i32)
        .bind(&tenant.database_schema)
        .execute(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        // Create dedicated schema for tenant's data
        let schema_name = &tenant.database_schema;
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {}", schema_name))
            .execute(&self.pg_pool)
            .await
            .map_err(|e| Error::DatabaseError(e))?;

        info!("Created tenant {} with schema {}", tenant.id, schema_name);

        Ok(())
    }

    /// Create new branch
    pub async fn create_branch(
        &self,
        tenant_id: &TenantId,
        branch_id: &BranchId,
        name: &str,
        api_key_hash: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO branches (id, tenant_id, name, api_key_hash, status)
            VALUES ($1, $2, $3, $4, 'offline')
            "#
        )
        .bind(branch_id.as_str())
        .bind(tenant_id.as_str())
        .bind(name)
        .bind(api_key_hash)
        .execute(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        info!("Created branch {} for tenant {}", branch_id, tenant_id);

        Ok(())
    }

    /// Update branch status
    pub async fn update_branch_status(
        &self,
        tenant_id: &TenantId,
        branch_id: &BranchId,
        status: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE branches SET status = $1, updated_at = NOW() WHERE id = $2 AND tenant_id = $3"
        )
        .bind(status)
        .bind(branch_id.as_str())
        .bind(tenant_id.as_str())
        .execute(&self.pg_pool)
        .await
        .map_err(|e| Error::DatabaseError(e))?;

        Ok(())
    }
}

// Database row types
#[derive(Debug, sqlx::FromRow)]
struct TenantRow {
    id: String,
    name: String,
    company_name: String,
    contact_email: String,
    status: String,
    max_branches: i32,
    max_connections_per_branch: i32,
    rate_limit_per_sec: i32,
    database_schema: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<TenantRow> for Tenant {
    fn from(row: TenantRow) -> Self {
        Tenant {
            id: TenantId::new(row.id),
            name: row.name,
            company_name: row.company_name,
            contact_email: row.contact_email,
            status: match row.status.as_str() {
                "active" => common::TenantStatus::Active,
                "suspended" => common::TenantStatus::Suspended,
                "trial" => common::TenantStatus::Trial,
                _ => common::TenantStatus::Inactive,
            },
            max_branches: row.max_branches as usize,
            max_connections_per_branch: row.max_connections_per_branch as usize,
            rate_limit_per_sec: row.rate_limit_per_sec as u32,
            database_schema: row.database_schema,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct BranchRow {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub status: String,
    pub api_key_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<BranchRow> for BranchInfo {
    fn from(row: BranchRow) -> Self {
        BranchInfo {
            id: BranchId::new(row.id),
            name: row.name,
            location: String::new(), // TODO: Add location field
            status: match row.status.as_str() {
                "online" => common::BranchStatus::Online,
                "syncing" => common::BranchStatus::Syncing,
                "error" => common::BranchStatus::Error,
                _ => common::BranchStatus::Offline,
            },
            last_seen: row.updated_at,
            metadata: std::collections::HashMap::new(),
        }
    }
}
