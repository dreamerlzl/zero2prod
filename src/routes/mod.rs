pub mod health;
pub mod subscribe;

use poem::middleware::Tracing;
use poem::{get, Route};
use poem::{Endpoint, EndpointExt};
use sea_orm::Database;
use tracing::info;

use self::health::health_check;
use crate::configuration::Configuration;

pub async fn default_route(conf: Configuration) -> Route {
    let sql = &conf.db;
    let db_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        sql.username, sql.password, sql.host, sql.port, sql.name,
    );
    info!(db_url, "connecting to db");
    let db = Database::connect(db_url)
        .await
        .expect("fail to get sql db connection");

    let mut route = Route::new().at("/api/v1/health_check", get(health_check));

    let server_url = format!("http://localhost:{}", conf.app_port);
    let (api_service, ui) = subscribe::get_api_service(db, &server_url);
    route = route.nest("/", api_service).nest("/docs", ui);
    route
}

fn add_tracing(ep: impl Endpoint) -> impl Endpoint {
    ep.with(Tracing)
}
