use std::sync::Arc;

use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::{
    auth::register_test_user,
    configuration::Configuration,
    email_client::EmailClient,
    startup::{get_database_connection, get_email_client},
};

#[derive(Clone)]
pub struct StateContext {
    pub db: DatabaseConnection,
    pub email_client: Arc<EmailClient>,
    pub base_url: String,
}

impl StateContext {
    pub async fn new(conf: Configuration) -> Result<Self, anyhow::Error> {
        let db = get_database_connection(conf.db).await?;
        let salt = Uuid::new_v4().to_string();
        register_test_user(
            &db,
            &conf.app.admin_username,
            &conf.app.admin_password,
            &salt,
        )
        .await?;
        let email_client = Arc::new(get_email_client(conf.email_client)?);
        let base_url = conf.app.base_url;
        Ok(Self {
            db,
            // email_client: Arc::new(email_client),
            email_client,
            base_url,
        })
    }
}
