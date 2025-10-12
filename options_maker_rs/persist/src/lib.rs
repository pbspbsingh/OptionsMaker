use app_config::APP_CONFIG;

use sqlx::SqlitePool;
use sqlx::sqlite::{
    SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;

pub use sqlx::Error;
pub use sqlx::Result;

pub mod crawler;
pub mod groups;
pub mod price_level;
pub mod prices;
pub mod ticker;

static DB_POOL: OnceLock<SqlitePool> = OnceLock::new();

pub async fn init() -> Result<()> {
    let options = SqliteConnectOptions::from_str(&APP_CONFIG.db_url)?
        .auto_vacuum(SqliteAutoVacuum::Full)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .shared_cache(true)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .test_before_acquire(true)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    DB_POOL.set(pool).expect("failed to set DB pool");

    Ok(())
}

pub fn db() -> &'static SqlitePool {
    DB_POOL.get().expect("failed to get DB pool")
}
