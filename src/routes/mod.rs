use std::sync::Arc;

use poem::middleware::Tracing;
use poem::{get, Route};
use poem::{Endpoint, EndpointExt};

use self::health::health_check;
use crate::configuration::Configuration;
use crate::context::StateContext;

mod error;
pub mod health;
pub mod newsletters;
pub mod subscriptions;
pub use error::ApiErrorResponse;

pub async fn default_route(conf: Configuration, context: Arc<StateContext>) -> Route {
    let mut route = Route::new().at("/api/v1/health_check", get(health_check));

    let server_url = format!("http://localhost:{}", conf.app.port);

    // load subscriptions routing
    let (subscriptions_service, ui) =
        subscriptions::get_api_service(context.clone(), &format!("{server_url}/subscriptions"));
    route = route
        .nest("/subscriptions", subscriptions_service)
        .nest("/subscriptions/docs", ui);

    // load newsletters routing
    let (newsletters_service, ui) =
        newsletters::get_api_service(context, &format!("{server_url}/newsletters"));
    route = route
        .nest("/newsletters", newsletters_service)
        .nest("/newsletters/docs", ui);
    route
}

fn add_tracing(ep: impl Endpoint) -> impl Endpoint {
    ep.with(Tracing)
}
