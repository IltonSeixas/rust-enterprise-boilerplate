use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::AuditEventType;

#[derive(Debug, Clone)]
pub struct AuditEvent {
    id: Uuid,
    event_type: AuditEventType,
    actor_id: Option<Uuid>,
    target_id: Option<Uuid>,
    detail: String,
    occurred_at: DateTime<Utc>,
}

impl AuditEvent {
    pub fn new(
        event_type: AuditEventType,
        actor_id: Option<Uuid>,
        target_id: Option<Uuid>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            actor_id,
            target_id,
            detail: detail.into(),
            occurred_at: Utc::now(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn event_type(&self) -> AuditEventType {
        self.event_type
    }

    pub fn actor_id(&self) -> Option<Uuid> {
        self.actor_id
    }

    pub fn target_id(&self) -> Option<Uuid> {
        self.target_id
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}
