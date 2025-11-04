use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant ID - Her müşteri için unique identifier
/// Bu ID ile tüm data isolation sağlanır
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(format!("tenant_{}", Uuid::new_v4()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub company_name: String,
    pub contact_email: String,
    pub status: TenantStatus,
    pub max_branches: usize,
    pub max_connections_per_branch: usize,
    pub rate_limit_per_sec: u32,
    pub database_schema: String,  // PostgreSQL schema for this tenant
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantStatus {
    Active,
    Suspended,
    Inactive,
    Trial,
}

/// Full qualified branch identifier with tenant isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QualifiedBranchId {
    pub tenant_id: TenantId,
    pub branch_id: super::BranchId,
}

impl QualifiedBranchId {
    pub fn new(tenant_id: TenantId, branch_id: super::BranchId) -> Self {
        Self { tenant_id, branch_id }
    }

    /// Format: tenant_id:branch_id
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.tenant_id.0, self.branch_id.0)
    }

    /// Parse from string format
    pub fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            Some(Self {
                tenant_id: TenantId::new(parts[0]),
                branch_id: super::BranchId::new(parts[1]),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualified_branch_id() {
        let tenant_id = TenantId::new("tenant_123");
        let branch_id = super::BranchId::new("branch_456");
        let qid = QualifiedBranchId::new(tenant_id, branch_id);

        let serialized = qid.to_string();
        assert_eq!(serialized, "tenant_123:branch_456");

        let parsed = QualifiedBranchId::from_string(&serialized).unwrap();
        assert_eq!(parsed, qid);
    }
}
