use sqlx::PgPool;
use protocol::DatabaseChange;
use common::Result;

/// Replication engine applies changes from remote branches
pub struct ReplicationEngine {
    pool: PgPool,
}

impl ReplicationEngine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Apply a batch of changes to local database
    pub async fn apply_changes(&self, schema: &str, changes: Vec<DatabaseChange>) -> Result<Vec<usize>> {
        let mut failed_indices = Vec::new();

        for (idx, change) in changes.iter().enumerate() {
            if let Err(e) = self.apply_single_change(schema, change).await {
                tracing::warn!("Failed to apply change {}: {}", idx, e);
                failed_indices.push(idx);
            }
        }

        Ok(failed_indices)
    }

    /// Apply single change
    async fn apply_single_change(&self, schema: &str, change: &DatabaseChange) -> Result<()> {
        match change.operation {
            protocol::Operation::Insert => self.apply_insert(schema, change).await,
            protocol::Operation::Update => self.apply_update(schema, change).await,
            protocol::Operation::Delete => self.apply_delete(schema, change).await,
        }
    }

    async fn apply_insert(&self, schema: &str, change: &DatabaseChange) -> Result<()> {
        // Generate INSERT query dynamically based on change.data
        // This is simplified - production code needs proper SQL generation
        let query = format!(
            "INSERT INTO {}.{} SELECT * FROM jsonb_populate_record(NULL::{}.{}, $1) ON CONFLICT DO NOTHING",
            schema, change.table_name, schema, change.table_name
        );

        sqlx::query(&query)
            .bind(&change.data)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn apply_update(&self, schema: &str, change: &DatabaseChange) -> Result<()> {
        // TODO: Implement UPDATE logic
        Ok(())
    }

    async fn apply_delete(&self, schema: &str, change: &DatabaseChange) -> Result<()> {
        // TODO: Implement DELETE logic
        Ok(())
    }
}
