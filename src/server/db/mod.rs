//! Database access layer (SSR only): connection pool, migrations, and the
//! repositories that persist the CRM domain in PostgreSQL.
//!
//! The pool lives in a process-wide [`OnceLock`] because the SSR branch of
//! [`crate::api_client`] calls the service/repository layer directly (it has no
//! Axum state handle to thread a pool through).

pub mod audit;
pub mod cases;
pub mod evidence;
pub mod grants;
pub mod ids;
pub mod messages;
pub mod mfa;
pub mod password_reset;
pub mod seed;
pub mod sessions;
pub mod settings;
pub mod throttle;
pub mod trusted_devices;
pub mod users;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::OnceLock;

static POOL: OnceLock<PgPool> = OnceLock::new();

/// The initialized connection pool. Panics if [`init`] has not run yet.
pub fn pool() -> &'static PgPool {
    POOL.get()
        .expect("database pool not initialized; call server::db::init() first")
}

/// Connect to `DATABASE_URL`, run migrations, and seed the database if it is
/// empty. Safe to call once at startup.
pub async fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL is not set (see .env / .env.example)")?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    POOL.set(pool)
        .map_err(|_| "database pool already initialized")?;

    Ok(())
}

/// Server-side "now" as `YYYY-MM-DD HH:MM` in local time, matching the display
/// format the browser produces via `state::now_stamp`.
pub fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}
