use sqlx::PgPool;

/// Applies all pending migrations under `./migrations`, holding a Postgres
/// advisory lock for the duration of the run. sqlx's migrator acquires this
/// lock internally, which is what allows multiple replicas to call this
/// function concurrently at startup without racing on the same DDL.
pub async fn run(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

#[cfg(test)]
mod tests {
    use super::run;
    use sqlx::postgres::PgPoolOptions;
    use sqlx::Row;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    const CONCURRENT_REPLICAS: usize = 8;

    #[tokio::test]
    async fn run_concurrent_replicas_apply_migrations_exactly_once_without_error() {
        let container = Postgres::default()
            .start()
            .await
            .expect("postgres container should start");

        let host_port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("postgres container should expose port 5432");
        let database_url = format!("postgres://postgres:postgres@127.0.0.1:{host_port}/postgres");

        // Each simulated replica gets its own pool, mirroring how independent
        // app instances would each open their own connection pool in production.
        let mut handles = Vec::with_capacity(CONCURRENT_REPLICAS);
        for _ in 0..CONCURRENT_REPLICAS {
            let database_url = database_url.clone();
            handles.push(tokio::spawn(async move {
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&database_url)
                    .await
                    .expect("replica should connect to postgres");
                run(&pool).await
            }));
        }

        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.expect("migration task should not panic");
            assert!(result.is_ok(), "replica {i} failed to migrate: {result:?}");
        }

        let verify_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("verification pool should connect to postgres");

        let users_count: i64 = sqlx::query(
            "SELECT count(*) AS count FROM information_schema.tables WHERE table_name = 'users'",
        )
        .fetch_one(&verify_pool)
        .await
        .expect("table count query should succeed")
        .get("count");
        assert_eq!(users_count, 1, "users table must exist exactly once");

        let audit_count: i64 = sqlx::query(
            "SELECT count(*) AS count FROM information_schema.tables WHERE table_name = 'audit_log'",
        )
        .fetch_one(&verify_pool)
        .await
        .expect("table count query should succeed")
        .get("count");
        assert_eq!(audit_count, 1, "audit_log table must exist exactly once");
    }
}
