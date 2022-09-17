pub mod health;
pub mod subscribe;

use crate::configuration::Configuration;
use poem::{get, Route};
use sea_orm::{ConnectionTrait, Database, DbBackend, DbErr};

use health::health_check;

pub async fn default_route(conf: Configuration) -> Route {
    let sql = &conf.db;
    let db_url = format!(
        "postgres:://{}:{}@{}:{}/{}",
        sql.username, sql.password, sql.host, sql.port, sql.name,
    );
    let db = Database::connect(db_url)
        .await
        .expect("fail to get sql db connection");

    let mut route = Route::new().at("/api/v1/health_check", get(health_check));

    let (api_service, ui) = subscribe::get_api_service();
    route = route.nest("/", api_service).nest("/docs", ui);

    route
}
