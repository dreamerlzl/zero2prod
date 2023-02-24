use poem_openapi::OpenApiService;

use crate::context::StateContext;

mod dashboard;

pub fn get_admin_service(
    context: StateContext,
    server_url: &str,
) -> (OpenApiService<dashboard::Api, ()>, ()) {
    let service =
        OpenApiService::new(dashboard::Api::new(context), "admin", "0.1").server(server_url);
    (service, ())
}
