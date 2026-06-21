CREATE TABLE IF NOT EXISTS audit_log (
    id          UUID PRIMARY KEY,
    event_type  TEXT NOT NULL,
    actor_id    UUID,
    target_id   UUID,
    detail      TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_log_occurred_at ON audit_log(occurred_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor_id ON audit_log(actor_id);
