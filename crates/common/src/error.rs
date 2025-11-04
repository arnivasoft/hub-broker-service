use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Authorization failed: {0}")]
    AuthorizationFailed(String),

    #[error("Invalid branch ID: {0}")]
    InvalidBranchId(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Redis error: {0}")]
    RedisError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Message routing error: {0}")]
    RoutingError(String),

    #[error("Sync conflict detected: {0}")]
    SyncConflict(String),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;

// Implement From for redis::RedisError
impl From<redis::RedisError> for Error {
    fn from(err: redis::RedisError) -> Self {
        Error::RedisError(err.to_string())
    }
}
