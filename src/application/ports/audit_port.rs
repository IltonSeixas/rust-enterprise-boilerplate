use async_trait::async_trait;

use crate::domain::audit::AuditEvent;

/// Implementations must never fail the use case they observe — on any
/// underlying error, log and degrade gracefully rather than propagating.
#[async_trait]
pub trait AuditPort: Send + Sync {
    async fn record(&self, event: AuditEvent);
}
