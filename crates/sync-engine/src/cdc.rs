use protocol::{DatabaseChange, Operation};
use sqlx::PgPool;
use common::Result;
use tracing::{debug, info};

/// Change Data Capture engine
///
/// Strategies:
/// 1. Trigger-based: Install triggers on tables to capture changes
/// 2. Logical replication: Use PostgreSQL logical replication slots
/// 3. Application-level: Track changes in application layer
pub struct CdcEngine {
    pool: PgPool,
    tracked_tables: Vec<String>,
}

impl CdcEngine {
    pub fn new(pool: PgPool, tracked_tables: Vec<String>) -> Self {
        Self {
            pool,
            tracked_tables,
        }
    }

    /// Install triggers on tracked tables for CDC
    pub async fn install_triggers(&self, schema: &str) -> Result<()> {
        info!("Installing CDC triggers for schema: {}", schema);

        // Create change log table
        let create_log_table = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}.sync_change_log (
                id BIGSERIAL PRIMARY KEY,
                table_name VARCHAR(255) NOT NULL,
                operation VARCHAR(10) NOT NULL,
                primary_key JSONB NOT NULL,
                row_data JSONB NOT NULL,
                changed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                synced BOOLEAN NOT NULL DEFAULT FALSE,
                branch_id VARCHAR(255) NOT NULL
            )
            "#,
            schema
        );

        sqlx::query(&create_log_table)
            .execute(&self.pool)
            .await?;

        // Create trigger function
        let trigger_function = format!(
            r#"
            CREATE OR REPLACE FUNCTION {}.log_changes()
            RETURNS TRIGGER AS $$
            BEGIN
                IF TG_OP = 'INSERT' THEN
                    INSERT INTO {}.sync_change_log (table_name, operation, primary_key, row_data, branch_id)
                    VALUES (TG_TABLE_NAME, 'INSERT', row_to_json(NEW)->'id', row_to_json(NEW), current_setting('app.branch_id', true));
                    RETURN NEW;
                ELSIF TG_OP = 'UPDATE' THEN
                    INSERT INTO {}.sync_change_log (table_name, operation, primary_key, row_data, branch_id)
                    VALUES (TG_TABLE_NAME, 'UPDATE', row_to_json(NEW)->'id', row_to_json(NEW), current_setting('app.branch_id', true));
                    RETURN NEW;
                ELSIF TG_OP = 'DELETE' THEN
                    INSERT INTO {}.sync_change_log (table_name, operation, primary_key, row_data, branch_id)
                    VALUES (TG_TABLE_NAME, 'DELETE', row_to_json(OLD)->'id', row_to_json(OLD), current_setting('app.branch_id', true));
                    RETURN OLD;
                END IF;
            END;
            $$ LANGUAGE plpgsql;
            "#,
            schema, schema, schema, schema
        );

        sqlx::query(&trigger_function)
            .execute(&self.pool)
            .await?;

        // Install triggers on each tracked table
        for table in &self.tracked_tables {
            let trigger_sql = format!(
                r#"
                DROP TRIGGER IF EXISTS sync_trigger ON {}.{};
                CREATE TRIGGER sync_trigger
                AFTER INSERT OR UPDATE OR DELETE ON {}.{}
                FOR EACH ROW EXECUTE FUNCTION {}.log_changes();
                "#,
                schema, table, schema, table, schema
            );

            sqlx::query(&trigger_sql)
                .execute(&self.pool)
                .await?;

            debug!("Installed trigger on {}.{}", schema, table);
        }

        info!("CDC triggers installed successfully");
        Ok(())
    }

    /// Fetch pending changes
    pub async fn fetch_pending_changes(&self, schema: &str, limit: i64) -> Result<Vec<DatabaseChange>> {
        let query = format!(
            r#"
            SELECT table_name, operation, primary_key, row_data, changed_at
            FROM {}.sync_change_log
            WHERE synced = FALSE
            ORDER BY id
            LIMIT $1
            "#,
            schema
        );

        let rows = sqlx::query_as::<_, ChangeLogRow>(&query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|row| row.into()).collect())
    }

    /// Mark changes as synced
    pub async fn mark_synced(&self, schema: &str, change_ids: &[i64]) -> Result<()> {
        let query = format!(
            r#"
            UPDATE {}.sync_change_log
            SET synced = TRUE
            WHERE id = ANY($1)
            "#,
            schema
        );

        sqlx::query(&query)
            .bind(change_ids)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ChangeLogRow {
    table_name: String,
    operation: String,
    primary_key: sqlx::types::JsonValue,
    row_data: sqlx::types::JsonValue,
    changed_at: chrono::DateTime<chrono::Utc>,
}

impl From<ChangeLogRow> for DatabaseChange {
    fn from(row: ChangeLogRow) -> Self {
        DatabaseChange {
            table_name: row.table_name,
            operation: match row.operation.as_str() {
                "INSERT" => Operation::Insert,
                "UPDATE" => Operation::Update,
                "DELETE" => Operation::Delete,
                _ => Operation::Insert,
            },
            primary_key: serde_json::Value::from(row.primary_key),
            data: serde_json::Value::from(row.row_data),
            timestamp: row.changed_at,
            schema_version: 1, // TODO: Track schema versions
        }
    }
}
