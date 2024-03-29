use std::convert::TryFrom;

use anyhow::Context;
use poem::Endpoint;
use poem_openapi::{
    param::Query,
    payload::{Form, Json},
    Object, OpenApi, OpenApiService,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sea_orm::*;
use serde::Deserialize;
use uuid::Uuid;

use super::{add_tracing, error::BasicError};
use crate::{
    context::StateContext,
    domain::{Email, UserName},
    entities::{prelude::*, subscription_tokens, subscriptions},
};

type SubscriptionResult<T> = std::result::Result<T, BasicError>;

pub struct Api {
    context: StateContext,
}

pub fn get_api_service(
    context: StateContext,
    server_url: &str,
) -> (OpenApiService<Api, ()>, impl Endpoint) {
    let api_service = OpenApiService::new(Api::new(context), "subscribe", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

impl Api {
    pub fn new(context: StateContext) -> Self {
        Self { context }
    }

    #[tracing::instrument(skip(conn))]
    async fn insert_subscriber<C>(
        conn: &C,
        new_subscriber: NewSubscriber,
    ) -> Result<Uuid, sea_orm::DbErr>
    where
        C: ConnectionTrait,
    {
        let new_subscription = subscriptions::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            name: ActiveValue::Set(new_subscriber.username.inner()),
            email: ActiveValue::Set(new_subscriber.email.inner()),
            status: ActiveValue::Set(ConfirmStatus::Pending.to_string()),
            ..Default::default()
        };
        let res = Subscriptions::insert(new_subscription).exec(conn).await?;
        Ok(res.last_insert_id)
    }

    #[tracing::instrument(skip(conn))]
    async fn store_subscription_token<C: ConnectionTrait>(
        conn: &C,
        subscriber_id: Uuid,
        token: String,
    ) -> Result<(), sea_orm::DbErr> {
        let new_subscription_token = subscription_tokens::ActiveModel {
            subscriber_id: ActiveValue::Set(subscriber_id),
            subscription_token: ActiveValue::Set(token),
        };
        _ = SubscriptionTokens::insert(new_subscription_token)
            .exec(conn)
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn send_subscription_email(
        &self,
        recipient: Email,
        token: &str,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let confirm_link = format!(
            "{}/subscriptions/confirm?token={}",
            self.context.base_url, token
        );
        self.context
            .email_client
            .send_email(
                &recipient,
                "welcome new subscriber",
                &format!("<a href=\"{confirm_link}\">here</a>"),
                &format!("{confirm_link}"),
            )
            .await
    }
}

#[OpenApi]
impl Api {
    // make a subscription
    #[oai(path = "/", method = "post", transform = "add_tracing")]
    #[tracing::instrument(
        skip(self, form),
        name = "new subscription",
        fields(
            email=%form.0.email,
            user=%form.0.username
        )
    )]
    async fn subscribe(
        &self,
        form: Form<SubscribeFormData>,
    ) -> SubscriptionResult<Json<CreateSuccess>> {
        let new_subscriber = NewSubscriber::try_from(form).map_err(BasicError::bad_request)?;
        let recipient = new_subscriber.email.clone();

        let txn = self
            .context
            .db
            .begin()
            .await
            .context("fail to init db transaction")
            .map_err(BasicError::interval_error)?;
        let last_insert_id = Api::insert_subscriber(&txn, new_subscriber)
            .await
            .context("fail to insert a new subscriber")
            .map_err(BasicError::interval_error)?;
        let subscription_token = generate_subscription_token();
        Api::store_subscription_token(&txn, last_insert_id, subscription_token.clone())
            .await
            .context("fail to store new subscription_token")
            .map_err(BasicError::interval_error)?;
        self.send_subscription_email(recipient, &subscription_token)
            .await
            .context("fail to send subscription email")
            .map_err(BasicError::interval_error)?;
        txn.commit().await.map_err(BasicError::interval_error)?;

        Ok(Json(CreateSuccess {
            id: last_insert_id.to_string(),
        }))
    }

    #[oai(path = "/confirm", method = "get", transform = "add_tracing")]
    #[tracing::instrument(skip(self, token), name = "new subscription confirm")]
    async fn confirm(&self, token: Query<String>) -> SubscriptionResult<()> {
        let subscription_token = token.0.clone();
        let subscriber_status = SubscriptionTokens::find()
            .filter(subscription_tokens::Column::SubscriptionToken.eq(subscription_token))
            .one(&self.context.db)
            .await
            .context("fail to find the subscriber with the specified token")
            .map_err(BasicError::interval_error)?;
        if subscriber_status.is_none() {
            let msg = format!("token {} not found in db", token.0);
            return Err(BasicError::bad_request(msg));
        }
        let subscriber_status = subscriber_status.unwrap();
        let subscriber_id = subscriber_status.subscriber_id;
        let subscriber = Subscriptions::find_by_id(subscriber_id)
            .one(&self.context.db)
            .await
            .context("fail to find subscriber with id")
            .map_err(BasicError::interval_error)?;
        match subscriber {
            Some(subscriber) => {
                let mut subscriber: subscriptions::ActiveModel = subscriber.into();
                subscriber.status = Set(ConfirmStatus::Confirmed.to_string());
                subscriber
                    .update(&self.context.db)
                    .await
                    .context("fail to update the subscriber status")
                    .map_err(BasicError::interval_error)?;
                Ok(())
            }
            None => {
                let msg = format!(
                    "fail to find subscriber despite foreign key contraint: {}",
                    subscriber_id
                );
                Err(BasicError::interval_error(msg))
            }
        }
    }
}

#[derive(Deserialize, Debug, Object)]
struct SubscribeFormData {
    username: String,
    email: String,
}

#[derive(Debug)]
struct NewSubscriber {
    username: UserName,
    email: Email,
}

impl TryFrom<Form<SubscribeFormData>> for NewSubscriber {
    type Error = String;
    fn try_from(form: Form<SubscribeFormData>) -> Result<Self, Self::Error> {
        Ok(NewSubscriber {
            username: UserName::parse(&form.0.username)?,
            email: Email::parse(form.0.email.clone())?,
        })
    }
}

#[derive(Object)]
struct CreateSuccess {
    id: String,
}

pub enum ConfirmStatus {
    Pending,
    Confirmed,
}

impl ToString for ConfirmStatus {
    fn to_string(&self) -> String {
        match self {
            ConfirmStatus::Pending => "pending_confirmed".to_owned(),
            ConfirmStatus::Confirmed => "confirmed".to_owned(),
        }
    }
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
