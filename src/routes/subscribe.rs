use poem::Endpoint;
use poem_openapi::{payload::Form, ApiResponse, Object, OpenApi, OpenApiService};
use serde::Deserialize;

pub struct SubscribeApi {}

pub fn get_api_service() -> (OpenApiService<SubscribeApi, ()>, impl Endpoint) {
    let api_service = OpenApiService::new(SubscribeApi::new(), "subscribe", "0.1").server("");
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

impl SubscribeApi {
    pub fn new() -> Self {
        todo!()
    }
}

#[OpenApi]
impl SubscribeApi {
    // make a subscription
    #[oai(path = "/subscription", method = "post")]
    async fn subscribe(&self, form: Form<SubscribeFormData>) -> CreateSubscriptionResponse {
        todo!()
    }
}

#[derive(Deserialize, Debug, Object)]
struct SubscribeFormData {
    user: String,
    email: String,
}

#[derive(ApiResponse)]
enum CreateSubscriptionResponse {
    #[oai(status = 200)]
    Ok,

    #[oai(status = 400)]
    InvalidData,
}
