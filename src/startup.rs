use anyhow::{Context, Result};
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use sea_orm_migration::prelude::*;
use sqlx::PgPool;
use tracing::info;

use crate::configuration::{EmailClientSettings, RelationalDBSettings};
use crate::email_client::EmailClient;

pub async fn get_database_connection(sql: RelationalDBSettings) -> Result<DatabaseConnection> {
    let db_url = format!(
        "postgres://{}:{:?}@{}:{}/{}",
        sql.username, sql.password, sql.host, sql.port, sql.name
    );
    info!(db_url, "connecting to db");

    let pg_options = sql.options_with_db();
    let pool = PgPool::connect_with(pg_options)
        .await
        .context("fail to connect to pg")?;
    let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
    Ok(db)
}

pub fn get_email_client(conf: EmailClientSettings) -> Result<EmailClient> {
    let sender_email = conf.sender().map_err(anyhow::Error::msg)?;
    let timeout = conf.timeout();
    let email_client = EmailClient::new(
        conf.api_base_url,
        sender_email,
        conf.authorization_token,
        timeout,
    );
    Ok(email_client)
}
