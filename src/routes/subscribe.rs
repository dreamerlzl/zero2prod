use crate::entities::{prelude::*, *};
use poem::Endpoint;
use poem_openapi::{
    payload::{Form, Json},
    ApiResponse, Object, OpenApi, OpenApiService,
};
use sea_orm::*;
use serde::Deserialize;
use validator::Validate;

pub struct SubscribeApi {
    db: DatabaseConnection,
}

pub fn get_api_service(
    db: DatabaseConnection,
) -> (OpenApiService<SubscribeApi, ()>, impl Endpoint) {
    let api_service = OpenApiService::new(SubscribeApi::new(db), "subscribe", "0.1").server("");
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
            return CreateSubscriptionResponse::InvalidData(Json(e.to_string()));
        }
        let new_subscription = subscription::ActiveModel {
            name: ActiveValue::Set(form.0.user),
            email: ActiveValue::Set(form.0.email),
            ..Default::default()
        };
        let res = Subscription::insert(new_subscription).exec(&self.db).await;
        if let Ok(record) = res {
            CreateSubscriptionResponse::Ok(Json(record.last_insert_id))
        } else {
            CreateSubscriptionResponse::ServerErr
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
