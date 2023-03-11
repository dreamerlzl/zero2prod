use poem::{
    get,
    middleware::{AddData, Tracing},
    post, Endpoint, EndpointExt, Route,
};

use self::{
    admin::{
        logout::post_logout,
        newsletters::{get_newsletter_submit_form, publish_newsletter},
    },
    health::health_check,
};
use crate::{auth::reject_anoynmous_user, configuration::Configuration, context::StateContext};

mod admin;
mod error;
pub mod health;
mod home;
mod login;
pub mod subscriptions;
pub use error::ApiErrorResponse;

pub async fn default_route(conf: Configuration, context: StateContext) -> Route {
    let mut route = Route::new()
        .at("/api/v1/health_check", get(health_check))
        .at("/logout", post(post_logout).around(reject_anoynmous_user))
        .at(
            "/admin/newsletters",
            post(publish_newsletter)
                .get(get_newsletter_submit_form)
                .around(reject_anoynmous_user)
                .with(Tracing)
                .with(AddData::new(context.clone())),
        )
        .at("/", get(home::home));

    let server_url = format!("http://127.0.0.1:{}", conf.app.port);

    // load subscriptions routing
    let (subscriptions_service, ui) =
        subscriptions::get_api_service(context.clone(), &format!("{server_url}/subscriptions"));
    route = route
        .nest("/subscriptions", subscriptions_service)
        .nest("/subscriptions/docs", ui);

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

fn add_session_uid_check(ep: impl Endpoint + 'static) -> impl Endpoint {
    ep.with(Tracing).around(reject_anoynmous_user)
}
