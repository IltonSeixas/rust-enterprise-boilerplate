#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[cfg(feature = "postgres")]
use crate::{application::ports::AuditPort, domain::audit::AuditEvent};

#[cfg(feature = "postgres")]
pub struct PostgresAuditLog {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresAuditLog {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AuditPort for PostgresAuditLog {
    async fn record(&self, event: AuditEvent) {
        let result = sqlx::query!(
            r#"INSERT INTO audit_log (id, event_type, actor_id, target_id, detail, occurred_at)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            event.id(),
            event.event_type().to_string(),
            event.actor_id(),
            event.target_id(),
            event.detail(),
            event.occurred_at(),
        )
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::error!(error = %e, audit_event_id = %event.id(), "failed to persist audit event");
        }
    }
}
