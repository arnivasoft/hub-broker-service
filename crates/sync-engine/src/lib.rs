//! PostgreSQL Sync Engine
//!
//! This crate handles:
//! - Change Data Capture (CDC) from PostgreSQL
//! - Conflict detection and resolution
//! - Transaction ordering with vector clocks
//! - Schema version management

pub mod cdc;
pub mod conflict;
pub mod replication;

pub use cdc::*;
pub use conflict::*;
pub use replication::*;
