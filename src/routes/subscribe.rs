use poem_openapi::{payload::Form, ApiResponse, Object, OpenApi, OpenApiService};
use serde::Deserialize;

pub struct SubscribeApi {}

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
