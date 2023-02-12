use std::sync::Arc;

use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHasher};
use base64::{engine::general_purpose, Engine};
use poem::{http::HeaderMap, web::Json, Endpoint};
use poem_openapi::{Object, OpenApi, OpenApiService};
use sea_orm::{ColumnTrait, DeriveColumn, EntityTrait, EnumIter, QueryFilter, QuerySelect};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use uuid::Uuid;

use super::{add_tracing, subscriptions::ConfirmStatus};
use crate::{
    context::StateContext,
    domain::Email,
    entities::{
        subscriptions::{self, Entity as Subscriptions},
        user::{self, Entity as Users},
    },
    routes::ApiErrorResponse,
};

pub struct Api {
    context: Arc<StateContext>,
}

pub fn get_api_service(
    context: Arc<StateContext>,
    server_url: &str,
) -> (OpenApiService<Api, ()>, impl Endpoint) {
    let api_service =
        OpenApiService::new(Api::new(context), "newsletters", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

#[OpenApi]
impl Api {
    #[tracing::instrument(name = "Publish a newsletter issue", skip(self), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
    #[oai(path = "/", method = "post", transform = "add_tracing")]
    async fn publish_newsletter(
        &self,
        headers: &HeaderMap,
        body: Json<NewsletterJsonPost>,
    ) -> Result<(), ApiErrorResponse> {
        // list all confirmed subscribers
        // ideally, we should let some workers to handle all the confirmed subscribers
        // asynchrounously
        let credentials = basic_authentication(headers).map_err(PublishError::AuthError)?;
        self.validate_credentials(credentials).await?;
        let subscribers = self.get_confirmed_subscribers().await?;
        for subscriber in subscribers {
            match subscriber {
                Ok(subscriber) => {
                    self.context
                        .email_client
                        .send_email(
                            &subscriber.email,
                            &body.title,
                            &body.content.html,
                            &body.content.text,
                        )
                        .await?;
                }
                Err(error) => {
                    tracing::warn!(error.cause_chain=?error, "skipping a confirmed subscriber due to invalid email stored")
                }
            }
        }
        Ok(())
    }
}

// {
//   "title": "abc",
//   "content": {
//     "html": "xxx",
//     "text": "yyy",
//   }
// }
#[derive(Deserialize, Debug, Object)]
struct NewsletterJsonPost {
    title: String,
    content: Content,
}

#[derive(Debug, Deserialize, Object)]
struct Content {
    html: String,
    text: String,
}

impl Api {
    fn new(context: Arc<StateContext>) -> Self {
        Api { context }
    }

    #[tracing::instrument(name = "get confirmed subscribers", skip(self))]
    async fn get_confirmed_subscribers(
        &self,
    ) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sea_orm::DbErr> {
        // select only one column without using a struct; ugly
        #[derive(Debug, Copy, Clone, EnumIter, DeriveColumn)]
        enum QueryAs {
            Email,
        }
        let subscribers = Subscriptions::find()
            .filter(subscriptions::Column::Status.eq(ConfirmStatus::Confirmed.to_string()))
            .select_only()
            .column(subscriptions::Column::Email)
            .into_values::<_, QueryAs>()
            .all(&self.context.db)
            .await?
            // when we first store the subscribers' email, the app could be version X
            // when we later fetch and parse the email, the app could be version Y
            // email validation logic may change between these 2 versions
            .into_iter()
            .map(|r| match Email::parse(r) {
                Ok(email) => Ok(ConfirmedSubscriber { email }),
                Err(err) => Err(anyhow!(err)),
            })
            .collect();
        Ok(subscribers)
    }

    async fn validate_credentials(&self, credentials: Credentials) -> Result<Uuid, PublishError> {
        let user = Users::find()
            .filter(user::Column::UserName.eq(credentials.username))
            .one(&self.context.db)
            .await
            .context("fail to find the auth user")?;

        let user = user.ok_or_else(|| anyhow!("invalid username or password"))?;
        let password_hash = get_hash(credentials.password.expose_secret(), &user.salt)
            .context("fail to validate credentials")?;
        if password_hash != user.password_hashed {
            return Err(anyhow!("invalid password").into());
        }
        Ok(user.id)
    }
}

struct ConfirmedSubscriber {
    email: Email,
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),

    #[error("unexpected error")]
    UnexpectedError(#[from] anyhow::Error),
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("the 'Authorization' header is missing")?
        .to_str()
        .context("the 'Authorization' header is not a valid utf8 str")?;

    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("the authorization scheme is not 'Basic'. ")?;
    let decoded_bytes = general_purpose::STANDARD.decode(base64encoded_segment)?;
    let decoded_credentials =
        String::from_utf8(decoded_bytes).context("the decoded credentials is not valid utf8")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("a username must be provided in 'Basic' auth"))?
        .to_owned();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow!("a password must be provided in 'Basic' auth"))?
        .to_owned();
    let password = Secret::new(password);
    Ok(Credentials { username, password })
}

pub fn get_hash(input: &String, salt: &String) -> anyhow::Result<String> {
    let argon2 = Argon2::default();
    // let salt = SaltString::generate(&mut OsRng);
    let salt = general_purpose::STANDARD.encode(salt);
    let password_hash = argon2
        .hash_password(input.as_bytes(), &salt)
        .map_err(|e| anyhow!(format!("fail to hash with argon2: {}", e.to_string())))?
        .to_string();
    Ok(password_hash)
}