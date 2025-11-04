use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub tenant_id: String,
    pub branch_id: String,
    pub api_key: String,
    pub hub_url: String,
    pub local_database_url: String,
    pub database_schema: String,
    pub tracked_tables: Vec<String>,
    pub sync_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            tenant_id: std::env::var("TENANT_ID")
                .expect("TENANT_ID must be set"),
            branch_id: std::env::var("BRANCH_ID")
                .expect("BRANCH_ID must be set"),
            api_key: std::env::var("API_KEY")
                .expect("API_KEY must be set"),
            hub_url: std::env::var("HUB_URL")
                .unwrap_or_else(|_| "ws://localhost:8080/ws".to_string()),
            local_database_url: std::env::var("LOCAL_DATABASE_URL")
                .expect("LOCAL_DATABASE_URL must be set"),
            database_schema: std::env::var("DATABASE_SCHEMA")
                .unwrap_or_else(|_| "public".to_string()),
            tracked_tables: std::env::var("TRACKED_TABLES")
                .unwrap_or_else(|_| "".to_string())
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect(),
            sync_interval_secs: std::env::var("SYNC_INTERVAL")
                .unwrap_or_else(|_| "30".to_string())
                .parse()?,
        })
    }
}
