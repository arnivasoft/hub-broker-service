use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Unique identifier for each branch/client
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BranchId(pub String);

impl BranchId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for BranchId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for BranchId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Branch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub id: BranchId,
    pub name: String,
    pub location: String,
    pub status: BranchStatus,
    pub last_seen: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BranchStatus {
    Online,
    Offline,
    Syncing,
    Error,
}

/// Vector clock for distributed conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorClock {
    pub clocks: HashMap<BranchId, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&mut self, branch_id: &BranchId) {
        *self.clocks.entry(branch_id.clone()).or_insert(0) += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (branch_id, &clock) in &other.clocks {
            let entry = self.clocks.entry(branch_id.clone()).or_insert(0);
            *entry = (*entry).max(clock);
        }
    }

    /// Returns true if self happened before other
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut less_than = false;
        for (branch_id, &other_clock) in &other.clocks {
            let self_clock = self.clocks.get(branch_id).copied().unwrap_or(0);
            if self_clock > other_clock {
                return false;
            }
            if self_clock < other_clock {
                less_than = true;
            }
        }
        less_than
    }

    /// Returns true if the clocks are concurrent (conflict)
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }
}

/// Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub branch_id: BranchId,
    pub api_key: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Connection metadata
#[derive(Debug, Clone)]
pub struct ConnectionMetadata {
    pub branch_id: BranchId,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub message_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_happens_before() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        let branch_a = BranchId::new("branch_a");
        let branch_b = BranchId::new("branch_b");

        clock1.increment(&branch_a);
        clock2.clocks = clock1.clocks.clone();
        clock2.increment(&branch_b);

        assert!(clock1.happens_before(&clock2));
        assert!(!clock2.happens_before(&clock1));
    }

    #[test]
    fn test_vector_clock_concurrent() {
        let mut clock1 = VectorClock::new();
        let mut clock2 = VectorClock::new();

        let branch_a = BranchId::new("branch_a");
        let branch_b = BranchId::new("branch_b");

        clock1.increment(&branch_a);
        clock2.increment(&branch_b);

        assert!(clock1.is_concurrent(&clock2));
        assert!(clock2.is_concurrent(&clock1));
    }
}
