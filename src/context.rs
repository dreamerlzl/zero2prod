use sea_orm::DatabaseConnection;

use crate::configuration::Configuration;
use crate::email_client::EmailClient;
use crate::startup::{get_database_connection, get_email_client};

#[derive(Clone)]
pub struct Context {
    pub db: DatabaseConnection,
    pub email_client: EmailClient,
}

impl Context {
    pub async fn new(conf: Configuration) -> Result<Self, anyhow::Error> {
        let db = get_database_connection(conf.db).await?;
        let email_client = get_email_client(conf.email_client)?;
        Ok(Self {
            db,
            // email_client: Arc::new(email_client),
            email_client,
        })
    }
}
