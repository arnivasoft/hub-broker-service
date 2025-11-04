use common::{VectorClock, Result};
use protocol::{DatabaseChange, ConflictStrategy, ConflictResolutionType};

/// Conflict detector and resolver
pub struct ConflictResolver {
    default_strategy: ConflictStrategy,
}

impl ConflictResolver {
    pub fn new(default_strategy: ConflictStrategy) -> Self {
        Self { default_strategy }
    }

    /// Detect if two changes conflict
    pub fn detect_conflict(
        &self,
        change_a: &DatabaseChange,
        change_b: &DatabaseChange,
        clock_a: &VectorClock,
        clock_b: &VectorClock,
    ) -> bool {
        // Same table and primary key
        if change_a.table_name != change_b.table_name {
            return false;
        }

        if change_a.primary_key != change_b.primary_key {
            return false;
        }

        // Check if concurrent (conflict)
        clock_a.is_concurrent(clock_b)
    }

    /// Resolve conflict using configured strategy
    pub fn resolve_conflict(
        &self,
        change_a: &DatabaseChange,
        change_b: &DatabaseChange,
        _clock_a: &VectorClock,
        _clock_b: &VectorClock,
    ) -> Result<(DatabaseChange, ConflictResolutionType)> {
        match self.default_strategy {
            ConflictStrategy::LastWriteWins => {
                // Compare timestamps
                if change_a.timestamp > change_b.timestamp {
                    Ok((change_a.clone(), ConflictResolutionType::LocalWins))
                } else {
                    Ok((change_b.clone(), ConflictResolutionType::RemoteWins))
                }
            }
            ConflictStrategy::FirstWriteWins => {
                if change_a.timestamp < change_b.timestamp {
                    Ok((change_a.clone(), ConflictResolutionType::LocalWins))
                } else {
                    Ok((change_b.clone(), ConflictResolutionType::RemoteWins))
                }
            }
            ConflictStrategy::ManualResolution => {
                // Store for manual resolution
                Err(common::Error::SyncConflict(
                    "Manual resolution required".to_string(),
                ))
            }
            ConflictStrategy::MergeFields => {
                // Merge non-conflicting fields
                // This requires field-level comparison
                self.merge_changes(change_a, change_b)
            }
        }
    }

    /// Merge changes at field level
    fn merge_changes(
        &self,
        change_a: &DatabaseChange,
        change_b: &DatabaseChange,
    ) -> Result<(DatabaseChange, ConflictResolutionType)> {
        // TODO: Implement smart field-level merging
        // For now, fall back to last-write-wins
        if change_a.timestamp > change_b.timestamp {
            Ok((change_a.clone(), ConflictResolutionType::Merged))
        } else {
            Ok((change_b.clone(), ConflictResolutionType::Merged))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_detection() {
        // Add comprehensive conflict detection tests
    }
}
