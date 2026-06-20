pub mod in_memory_audit_log;
pub mod postgres_audit_log;

#[cfg(not(feature = "postgres"))]
pub use in_memory_audit_log::InMemoryAuditLog;

#[cfg(feature = "postgres")]
pub use postgres_audit_log::PostgresAuditLog;
