use std::str::FromStr;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use super::migrations::MIGRATIONS;

pub type Db = SqlitePool;

pub async fn connect_and_migrate(url: &str) -> anyhow::Result<Db> {
    let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    for statement in MIGRATIONS {
        if let Err(error) = sqlx::query(statement).execute(&pool).await {
            let message = error.to_string();
            if statement.starts_with("ALTER TABLE")
                && statement.contains(" ADD COLUMN ")
                && message.contains("duplicate column")
            {
                continue;
            }
            return Err(error.into());
        }
    }

    Ok(pool)
}
