use crate::entities::{prelude::*, *};
use chrono::{DateTime, Local};
use poem::Endpoint;
use poem_openapi::{
    payload::{Form, Json},
    ApiResponse, Object, OpenApi, OpenApiService,
};
use sea_orm::*;
use serde::Deserialize;
use tracing::{debug, info, warn};
use validator::Validate;

pub struct SubscribeApi {
    db: DatabaseConnection,
}

pub fn get_api_service(
    db: DatabaseConnection,
    server_url: &str,
) -> (OpenApiService<SubscribeApi, ()>, impl Endpoint) {
    let api_service =
        OpenApiService::new(SubscribeApi::new(db), "subscribe", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

impl SubscribeApi {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[OpenApi]
impl SubscribeApi {
    // make a subscription
    #[oai(path = "/subscription", method = "post")]
    async fn subscribe(&self, form: Form<SubscribeFormData>) -> CreateSubscriptionResponse {
        if let Err(e) = form.0.validate() {
            info!(error = e.to_string());
            return CreateSubscriptionResponse::InvalidData(Json(e.to_string()));
        }
        let new_subscription = subscription::ActiveModel {
            name: ActiveValue::Set(form.0.user),
            email: ActiveValue::Set(form.0.email),
            ..Default::default()
        };
        let res = Subscription::insert(new_subscription).exec(&self.db).await;
        match res {
            Ok(record) => {
                debug!(record.last_insert_id, "newly created subscription id");
                CreateSubscriptionResponse::Ok(Json(record.last_insert_id))
            }
            Err(e) => {
                warn!(error = e.to_string());
                CreateSubscriptionResponse::ServerErr
            }
        }
    }
}

#[derive(Deserialize, Debug, Object, Validate)]
struct SubscribeFormData {
    user: String,
    #[validate(email)]
    email: String,
}

#[derive(ApiResponse)]
enum CreateSubscriptionResponse {
    #[oai(status = 200)]
    Ok(Json<i32>),

    #[oai(status = 400)]
    InvalidData(Json<String>),

    #[oai(status = 500)]
    ServerErr,
}
