use anyhow::Result;
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use sea_orm_migration::prelude::*;
use sqlx::PgPool;
use tracing::info;

use crate::configuration::Configuration;

pub async fn get_database_connection(conf: &Configuration) -> Result<DatabaseConnection> {
    let sql = &conf.db;
    let db_url = format!(
        "postgres://{}:{:?}@{}:{}/{}",
        sql.username, sql.password, sql.host, sql.port, sql.name
    );
    info!(db_url, "connecting to db");

    let pg_options = sql.options_with_db();
    let pool = PgPool::connect_with(pg_options).await?;
    let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
    Ok(db)
}
