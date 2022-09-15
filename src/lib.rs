pub mod configuration;
pub mod routes;

use configuration::Configuration;
use poem::{get, handler, post, web::Path, Route};
use sea_orm::{ConnectionTrait, Database, DbBackend, DbErr};

use routes::subscribe;

#[handler]
pub fn hello(Path(name): Path<String>) -> String {
    format!("hello {}", name)
}

#[handler]
pub fn health_check() {
    // () -> 200 OK in poem
}

pub async fn default_route(conf: Configuration) -> Route {
    let sql = &conf.db;
    let db_url = format!(
        "postgres:://{}:{}@{}:{}/{}",
        sql.username, sql.password, sql.host, sql.port, sql.name,
    );
    let db = Database::connect(db_url)
        .await
        .expect("fail to get sql db connection");

    let mut route = Route::new()
        .at("/api/v1/hello/:name", get(hello))
        .at("/api/v1/health_check", get(health_check));

    let (api_service, ui) = self::routes::get_api_service();
    route = route.nest("/", api_service).nest("/docs", ui);

    route
}
