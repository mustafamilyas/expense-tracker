use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use anyhow::Result;

pub async fn make_db_pool(db_url: &str) -> Result<sqlx::PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(db_url)
        .await?;
    Ok(pool)
}