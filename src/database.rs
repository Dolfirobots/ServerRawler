use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::OnceLock;
use std::time::Duration;

static DB_POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

pub struct DatabaseManager;

impl DatabaseManager {
    pub async fn init(url: &str) -> Result<(), String> {
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .acquire_timeout(Duration::from_secs(5))
            .connect(url)
            .await
            .map_err(|e| format!("Failed to establish database connection: {}", e))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| format!("Database migration failed: {}", e))?;

        DB_POOL
            .set(pool)
            .map_err(|_| "Database pool has already been initialized!".to_string())?;

        Ok(())
    }

    pub fn get_pool() -> &'static Pool<Postgres> {
        DB_POOL.get().expect("Database pool not initialized!")
    }
}