use std::{convert::TryFrom, sync::Arc};

use poem::Endpoint;
use poem_openapi::{
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
    entities::{prelude::*, *},
};

pub struct SubscribeApi {
    context: Arc<Context>,
}

pub fn get_api_service(
    context: Context,
    server_url: &str,
) -> (OpenApiService<SubscribeApi, ()>, impl Endpoint) {
    let api_service =
        OpenApiService::new(SubscribeApi::new(context), "subscribe", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

impl SubscribeApi {
    pub fn new(context: Context) -> Self {
        Self {
            context: Arc::new(context),
        }
    }
}

#[OpenApi]
impl SubscribeApi {
    // make a subscription
    #[oai(path = "/subscription", method = "post", transform = "add_tracing")]
    #[tracing::instrument(
        skip(self, form),
        name = "new subscription",
        fields(
            email=%form.0.email,
            user=%form.0.username
        )
    )]
    async fn subscribe(&self, form: Form<SubscribeFormData>) -> CreateSubscriptionResponse {
        self.context.email_client.hello().await;
        let new_subscriber = match NewSubscriber::try_from(form) {
            Err(e) => {
                return CreateSubscriptionResponse::InvalidData(Json(InvalidData {
                    msg: e.to_string(),
                }));
            }
            Ok(new_subscriber) => new_subscriber,
        };
        let res = self.insert_subscriber(new_subscriber).await;
        match res {
            Ok(last_insert_id) => {
                info!(last_insert_id, "newly created subscription id");
                CreateSubscriptionResponse::Ok(Json(CreateSuccess { id: last_insert_id }))
            }
            Err(e) => {
                warn!(error = e.to_string());
                CreateSubscriptionResponse::ServerErr
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn insert_subscriber(
        &self,
        new_subscriber: NewSubscriber,
    ) -> Result<i32, sea_orm::DbErr> {
        let new_subscription = subscription::ActiveModel {
            name: ActiveValue::Set(new_subscriber.username.inner()),
            email: ActiveValue::Set(new_subscriber.email.inner()),
            ..Default::default()
        };
        let res = Subscription::insert(new_subscription)
            .exec(&self.context.db)
            .await?;
        Ok(res.last_insert_id)
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
