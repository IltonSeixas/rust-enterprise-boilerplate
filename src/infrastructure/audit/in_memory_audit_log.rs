use async_trait::async_trait;

use crate::{application::ports::AuditPort, domain::audit::AuditEvent};

#[cfg_attr(feature = "postgres", allow(dead_code))]
pub struct InMemoryAuditLog;

#[cfg_attr(feature = "postgres", allow(dead_code))]
impl InMemoryAuditLog {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InMemoryAuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditPort for InMemoryAuditLog {
    async fn record(&self, event: AuditEvent) {
        tracing::info!(
            audit_event_id = %event.id(),
            event_type = %event.event_type(),
            actor_id = ?event.actor_id(),
            target_id = ?event.target_id(),
            detail = %event.detail(),
            occurred_at = %event.occurred_at(),
            "audit event recorded"
        );
    }
}
