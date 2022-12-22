use std::sync::Arc;

use poem::middleware::Tracing;
use poem::{get, Route};
use poem::{Endpoint, EndpointExt};

use self::health::health_check;
use crate::configuration::Configuration;
use crate::context::Context;

pub mod health;
pub mod subscriptions;

pub async fn default_route(conf: Configuration, context: Arc<Context>) -> Route {
    let mut route = Route::new().at("/api/v1/health_check", get(health_check));

    let server_url = format!("http://localhost:{}", conf.app.port);
    let (subscriptions_service, ui) =
        subscriptions::get_api_service(context.clone(), &format!("{}/subscriptions", server_url));
    route = route
        .nest("/subscriptions", subscriptions_service)
        .nest("/subscriptions/docs", ui);
    route
}

fn add_tracing(ep: impl Endpoint) -> impl Endpoint {
    ep.with(Tracing)
}
