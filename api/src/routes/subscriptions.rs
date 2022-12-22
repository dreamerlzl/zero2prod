use std::{convert::TryFrom, sync::Arc};

use poem::Endpoint;
use poem_openapi::{
    param::Query,
    payload::{Form, Json},
    ApiResponse, Object, OpenApi, OpenApiService,
};
use sea_orm::*;
use serde::Deserialize;
use tracing::{info, warn};

use super::add_tracing;
use crate::{
    context::Context,
    domain::{Email, UserName},
    entities::{prelude::*, subscriptions},
};

pub const DEFAULT_CONFIRM_STATUS: &'static str = "pending_confirmed";

pub struct Api {
    context: Arc<Context>,
}

pub fn get_api_service(
    context: Arc<Context>,
    server_url: &str,
) -> (OpenApiService<Api, ()>, impl Endpoint) {
    let api_service = OpenApiService::new(Api::new(context), "subscribe", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

impl Api {
    pub fn new(context: Arc<Context>) -> Self {
        Self { context }
    }

    #[tracing::instrument(skip(self))]
    async fn insert_subscriber(
        &self,
        new_subscriber: NewSubscriber,
    ) -> Result<i32, sea_orm::DbErr> {
        let new_subscription = subscriptions::ActiveModel {
            name: ActiveValue::Set(new_subscriber.username.inner()),
            email: ActiveValue::Set(new_subscriber.email.inner()),
            status: ActiveValue::Set(DEFAULT_CONFIRM_STATUS.to_owned()),
            ..Default::default()
        };
        let res = Subscriptions::insert(new_subscription)
            .exec(&self.context.db)
            .await?;
        Ok(res.last_insert_id)
    }

    async fn send_subscription_email(
        &self,
        recipient: Email,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let confirm_link = format!("{}/subscriptions/confirm", self.context.base_url);
        self.context
            .email_client
            .send_email(
                &recipient,
                "welcome new subscriber",
                &format!("<a href=\"{}\">here</a>", confirm_link),
                &format!("{}", confirm_link),
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
    async fn subscribe(&self, form: Form<SubscribeFormData>) -> CreateSubscriptionResponse {
        let new_subscriber = match NewSubscriber::try_from(form) {
            Err(e) => {
                return CreateSubscriptionResponse::InvalidData(Json(InvalidData { msg: e }));
            }
            Ok(new_subscriber) => new_subscriber,
        };
        let recipient = new_subscriber.email.clone();
        let res = self.insert_subscriber(new_subscriber).await;
        match res {
            Ok(last_insert_id) => {
                if let Err(e) = self.send_subscription_email(recipient).await {
                    warn!(error = e.to_string());
                    return CreateSubscriptionResponse::ServerErr;
                }
                info!(last_insert_id, "newly created subscription id");
                CreateSubscriptionResponse::Ok(Json(CreateSuccess { id: last_insert_id }))
            }
            Err(e) => {
                warn!(error = e.to_string());
                CreateSubscriptionResponse::ServerErr
            }
        }
    }

    #[oai(path = "/confirm", method = "get", transform = "add_tracing")]
    #[tracing::instrument(skip(self, token), name = "new subscription confirm")]
    async fn confirm(&self, token: Query<String>) -> ConfirmSubscriptionResponse {
        ConfirmSubscriptionResponse::Ok
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
    id: i32,
}

#[derive(Object)]
struct InvalidData {
    msg: String,
}

#[derive(ApiResponse)]
enum CreateSubscriptionResponse {
    #[oai(status = 200)]
    Ok(Json<CreateSuccess>),

    #[oai(status = 400)]
    InvalidData(Json<InvalidData>),

    #[oai(status = 500)]
    ServerErr,
}

#[derive(ApiResponse)]
enum ConfirmSubscriptionResponse {
    #[oai(status = 200)]
    Ok,
}
