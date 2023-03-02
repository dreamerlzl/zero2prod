use anyhow::anyhow;
use poem::http::HeaderMap;
use poem_openapi::{payload::Form, Object, OpenApi};
use sea_orm::{ColumnTrait, DeriveColumn, EntityTrait, EnumIter, QueryFilter, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use super::super::{add_session_uid_check, subscriptions::ConfirmStatus};
use crate::{
    auth::{validate_credentials, AuthError, Credentials},
    context::StateContext,
    domain::Email,
    entities::subscriptions::{self, Entity as Subscriptions},
    routes::ApiErrorResponse,
};

pub struct Api {
    context: StateContext,
}

#[OpenApi]
impl Api {
    #[tracing::instrument(name = "Publish a newsletter issue", skip(self), fields(username=tracing::field::Empty, user_id=tracing::field::Empty))]
    #[oai(
        path = "/newsletters",
        method = "post",
        transform = "add_session_uid_check"
    )]
    async fn publish_newsletter(
        &self,
        headers: &HeaderMap,
        body: Form<NewsletterForm>,
    ) -> Result<(), ApiErrorResponse> {
        // list all confirmed subscribers
        // ideally, we should let some workers to handle all the confirmed subscribers
        // asynchrounously
        let subscribers = self.get_confirmed_subscribers().await?;
        for subscriber in subscribers {
            match subscriber {
                Ok(subscriber) => {
                    self.context
                        .email_client
                        .send_email(
                            &subscriber.email,
                            &body.title,
                            &body.html_content,
                            &body.text_content,
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
struct NewsletterForm {
    title: String,
    text_content: String,
    html_content: String,
}

impl Api {
    pub fn new(context: StateContext) -> Self {
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

    #[tracing::instrument(name = "validate user's credentials", skip(self, credentials))]
    async fn validate_credentials(&self, credentials: Credentials) -> Result<Uuid, PublishError> {
        validate_credentials(&self.context.db, credentials)
            .await
            .map_err(|e| match e {
                AuthError::UnexpectedError(_) => PublishError::UnexpectedError(e.into()),
                AuthError::InvalidCredentials(_) => PublishError::AuthError(e.into()),
            })
    }
}

struct ConfirmedSubscriber {
    email: Email,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),

    #[error("unexpected error")]
    UnexpectedError(#[from] anyhow::Error),
}
