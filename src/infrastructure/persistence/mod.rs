pub mod in_memory_user_repository;
pub mod postgres_user_repository;

pub use in_memory_user_repository::InMemoryUserRepository;

#[cfg(feature = "postgres")]
pub use postgres_user_repository::PostgresUserRepository;
