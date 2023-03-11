mod dashboard;
pub mod logout;
pub mod newsletters;
mod password;

use poem::IntoEndpoint;
use poem_openapi::OpenApiService;

use crate::context::StateContext;

pub fn get_api_service(context: StateContext, server_url: &str) -> (impl IntoEndpoint, ()) {
    let service = OpenApiService::new(
        (
            dashboard::Api::new(context.clone()),
            password::Api::new(context),
        ),
        "admin",
        "0.1",
    )
    .server(server_url);
    (service, ())
}
