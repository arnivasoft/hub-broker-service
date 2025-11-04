use chrono::{DateTime, Utc};

/// Generate a unique transaction ID
pub fn generate_transaction_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Get current timestamp
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Calculate message hash for deduplication
pub fn calculate_hash(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Exponential backoff calculation
pub fn calculate_backoff_duration(attempt: u32, base_ms: u64, max_ms: u64) -> std::time::Duration {
    let backoff_ms = base_ms * 2u64.pow(attempt);
    let capped_ms = backoff_ms.min(max_ms);
    std::time::Duration::from_millis(capped_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_calculation() {
        assert_eq!(calculate_backoff_duration(0, 1000, 60000).as_millis(), 1000);
        assert_eq!(calculate_backoff_duration(1, 1000, 60000).as_millis(), 2000);
        assert_eq!(calculate_backoff_duration(2, 1000, 60000).as_millis(), 4000);
        assert_eq!(calculate_backoff_duration(10, 1000, 60000).as_millis(), 60000); // capped
    }
}
