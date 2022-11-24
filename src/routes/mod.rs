use poem::middleware::Tracing;
use poem::{get, Route};
use poem::{Endpoint, EndpointExt};

use self::health::health_check;
use crate::configuration::Configuration;
use crate::context::Context;

pub mod health;
pub mod subscribe;

pub async fn default_route(conf: Configuration, context: Context) -> Route {
    let mut route = Route::new().at("/api/v1/health_check", get(health_check));

    let server_url = format!("http://localhost:{}", conf.app_port);
    let (api_service, ui) = subscribe::get_api_service(context, &server_url);
    route = route.nest("/", api_service).nest("/docs", ui);
    route
}

fn add_tracing(ep: impl Endpoint) -> impl Endpoint {
    ep.with(Tracing)
}
