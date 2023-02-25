use poem::{get, middleware::Tracing, Endpoint, EndpointExt, Route};

use self::health::health_check;
use crate::{configuration::Configuration, context::StateContext};

mod admin;
mod error;
pub mod health;
mod home;
mod login;
pub mod newsletters;
pub mod subscriptions;
pub use error::ApiErrorResponse;

pub async fn default_route(conf: Configuration, context: StateContext) -> Route {
    let mut route = Route::new()
        .at("/api/v1/health_check", get(health_check))
        .at("/", get(home::home));

    let server_url = format!("http://localhost:{}", conf.app.port);

    // load subscriptions routing
    let (subscriptions_service, ui) =
        subscriptions::get_api_service(context.clone(), &format!("{server_url}/subscriptions"));
    route = route
        .nest("/subscriptions", subscriptions_service)
        .nest("/subscriptions/docs", ui);

    // load newsletters routing
    let (newsletters_service, ui) =
        newsletters::get_api_service(context.clone(), &format!("{server_url}/newsletters"));
    route = route
        .nest("/newsletters", newsletters_service)
        .nest("/newsletters/docs", ui);

    let (login_service, ui) =
        login::get_api_service(context.clone(), &format!("{server_url}/login"));
    route = route.nest("/login", login_service).nest("/login/docs", ui);

    let (admin_service, _) = admin::get_api_service(context, &format!("{server_url}/admin"));
    route = route.nest("/admin", admin_service);

    route
}

fn add_tracing(ep: impl Endpoint) -> impl Endpoint {
    ep.with(Tracing)
}
