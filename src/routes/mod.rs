pub mod subscribe;

use poem::Endpoint;
use poem_openapi::OpenApiService;

use crate::configuration::Configuration;

use self::subscribe::SubscribeApi;

pub fn get_api_service() -> (OpenApiService<SubscribeApi, ()>, impl Endpoint) {
    let api_service = OpenApiService::new(SubscribeApi::new(), "subscribe", "0.1").server("");
    let ui = api_service.swagger_ui();
    (api_service, ui)
}
