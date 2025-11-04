use common::{BranchId, TenantId, QualifiedBranchId, Result, Error};
use protocol::{Message, MessagePayload};
use crate::{websocket::ConnectionManager, storage::Storage};
use std::sync::Arc;
use tracing::{debug, warn, error};

/// Message router handles routing messages between branches
/// CRITICAL: Enforces tenant isolation - messages can only be routed within same tenant
pub struct MessageRouter {
    connection_manager: Arc<ConnectionManager>,
    storage: Storage,
}

impl MessageRouter {
    pub fn new(connection_manager: Arc<ConnectionManager>, storage: Storage) -> Self {
        Self {
            connection_manager,
            storage,
        }
    }

    /// Route message to appropriate destination
    /// ENFORCES: Tenant isolation
    pub async fn route_message(&self, message: Message) -> Result<()> {
        // Extract tenant_id from sender
        let sender_tenant = self.get_tenant_for_branch(&message.from).await?;

        // If message has a specific destination
        if let Some(ref target_branch) = message.to {
            // CRITICAL: Verify target branch belongs to same tenant
            let target_tenant = self.get_tenant_for_branch(target_branch).await?;

            if sender_tenant != target_tenant {
                error!(
                    "Cross-tenant routing attempt: {} -> {}",
                    sender_tenant, target_tenant
                );
                return Err(Error::AuthorizationFailed(
                    "Cannot route messages across tenants".to_string(),
                ));
            }

            // Route to specific branch
            self.forward_to_branch(target_branch, message).await?;
        } else {
            // Broadcast to all branches in same tenant
            self.broadcast_to_tenant(&sender_tenant, message, Some(&message.from))
                .await?;
        }

        Ok(())
    }

    /// Forward message to specific branch
    pub async fn forward_to_branch(&self, target: &BranchId, message: Message) -> Result<()> {
        if self.connection_manager.is_connected(target).await {
            self.connection_manager.send_message(target, message).await?;
            debug!("Message forwarded to {}", target);
        } else {
            // Store message for offline delivery
            warn!("Branch {} offline, storing message", target);
            self.store_offline_message(target, message).await?;
        }

        Ok(())
    }

    /// Broadcast message to all branches in a tenant
    /// ENFORCES: Only broadcasts within tenant boundary
    async fn broadcast_to_tenant(
        &self,
        tenant_id: &TenantId,
        message: Message,
        exclude: Option<&BranchId>,
    ) -> Result<()> {
        // Get all branches for this tenant
        let branches = self.storage.list_branches_for_tenant(tenant_id).await?;

        for branch in branches {
            // Skip excluded branch (usually sender)
            if let Some(exclude_id) = exclude {
                if &branch.id == exclude_id {
                    continue;
                }
            }

            // Only send to online branches
            if self.connection_manager.is_connected(&branch.id).await {
                if let Err(e) = self
                    .connection_manager
                    .send_message(&branch.id, message.clone())
                    .await
                {
                    warn!("Failed to send to {}: {}", branch.id, e);
                }
            }
        }

        Ok(())
    }

    /// Get tenant ID for a branch
    async fn get_tenant_for_branch(&self, branch_id: &BranchId) -> Result<TenantId> {
        // This should be cached in production
        self.storage.get_tenant_for_branch(branch_id).await
    }

    /// Store message for offline delivery
    async fn store_offline_message(&self, _target: &BranchId, _message: Message) -> Result<()> {
        // TODO: Implement Redis-based message queue
        // Messages should be stored with TTL
        Ok(())
    }

    /// Deliver pending offline messages when branch reconnects
    pub async fn deliver_offline_messages(&self, _branch_id: &BranchId) -> Result<()> {
        // TODO: Retrieve and deliver stored messages
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Add tests for routing logic
}
