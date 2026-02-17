use std::sync::OnceLock;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;
use colored_text::Colorize;
use sqlx::migrate::MigrateError;
use crate::{config, format_validation_report, logger, USE_DATABASE};
use crate::database::pool::DatabaseError::{ConnectionError, MigrationError, PoolAlreadyInitialized, Io, SqlxError};
use crate::logger::DefaultColor;

static DB_POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

// Error handling

#[derive(Debug)]
pub enum DatabaseError {
    Io(std::io::Error),
    MigrationError(MigrateError),
    ConnectionError(sqlx::Error),
    PoolAlreadyInitialized(),
    SqlxError(sqlx::Error)
}

impl DatabaseError {
    pub fn to_string(&self) -> String {
        let highlight = DefaultColor::Highlight.hex();
        match self {
            Io(e) => {
                format!(
                    "IO error: {}",
                    e.hex(highlight)
                )
            },
            MigrationError(e) => {
                format!(
                    "Migration error: {}",
                    e.hex(highlight)
                )
            },
            ConnectionError(e) => {
                format!(
                    "There was a connection error: {}",
                    e.hex(highlight)
                )
            },
            PoolAlreadyInitialized() => {
                "The database pool was already initialized!".into()
            },
            SqlxError(e) => {
                format!(
                    "Sqlx error: {}",
                    e.hex(highlight)
                )
            }
        }
    }
}

async fn init(url: &str) -> Result<(), DatabaseError> {
    let pool = PgPoolOptions::new()
        .max_connections(50) // Maybe make this configurable
        .acquire_timeout(Duration::from_secs(5))
        .connect(url)
        .await
        .map_err(ConnectionError)?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(MigrationError)?;

    DB_POOL
        .set(pool)
        .map_err(|_| PoolAlreadyInitialized())?;
    Ok(())
}

pub async fn load(config_path: Option<String>) {
    if !**USE_DATABASE.get().expect("Parser error at USE_DATABASE") {
        logger::warning(format!("Database functions are now {}!", "disabled".red()))
            .prefix("Database").send().await;
        return;
    }

    if let Err(err) = config::DatabaseConfig::load(config_path) {
        logger::critical(err.to_string()).prefix("Database").send().await;
        std::process::exit(1);
    }

    let db_cfg = config::DatabaseConfig::get().expect("DB Config must be loaded");
    let db_validation = db_cfg.validate();

    if !db_validation.is_empty() {
        let report = format_validation_report("Database configuration:", &db_validation);
        let has_critical = db_validation.iter().any(|e| e.is_critical());

        if has_critical {
            logger::critical(report).prefix("Database").send().await;
            std::process::exit(1);
        } else {
            logger::warning(report).prefix("Database").send().await;
        }
    }

    let db_cfg = config::DatabaseConfig::get().unwrap();

    if db_cfg.is_dangerous() {
        logger::warning(
            format!(
                "Database has weak credentials! See {}",
                "https://cyberdolfi.github.io/ServerRawler/docs/configuration/database#weak-credentials-warning".hex(DefaultColor::Highlight.hex())
            )
        ).prefix("Security").send().await;
    }

    if let Err(err) = init(&db_cfg.get_url()).await {
        logger::critical(err.to_string()).prefix("Database").send().await;
        std::process::exit(1);
    }
    logger::success("Connected to database".into()).prefix("Database").send().await;
}

pub fn get_pool() -> &'static Pool<Postgres> {
    DB_POOL.get().expect("Pool was't initialized!")
}