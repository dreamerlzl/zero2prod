use sea_orm::DatabaseConnection;

use crate::configuration::Configuration;
use crate::email_client::EmailClient;
use crate::startup::{get_database_connection, get_email_client};

#[derive(Clone)]
pub struct StateContext {
    pub db: DatabaseConnection,
    pub email_client: EmailClient,
    pub base_url: String,
}

impl StateContext {
    pub async fn new(conf: Configuration) -> Result<Self, anyhow::Error> {
        let db = get_database_connection(conf.db).await?;
        let email_client = get_email_client(conf.email_client)?;
        let base_url = conf.app.base_url;
        Ok(Self {
            db,
            // email_client: Arc::new(email_client),
            email_client,
            base_url,
        })
    }
}
