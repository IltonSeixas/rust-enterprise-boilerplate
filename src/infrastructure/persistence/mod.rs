pub mod in_memory_user_repository;
#[cfg(feature = "postgres")]
pub mod migrations;
pub mod postgres_user_repository;

#[cfg(not(feature = "postgres"))]
pub use in_memory_user_repository::InMemoryUserRepository;

#[cfg(feature = "postgres")]
pub use postgres_user_repository::PostgresUserRepository;
