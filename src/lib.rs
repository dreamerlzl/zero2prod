pub mod configuration;
pub mod routes;

use configuration::Configuration;
use poem::{get, handler, post, web::Path, Route};
use routes::subscribe;

#[handler]
pub fn hello(Path(name): Path<String>) -> String {
    format!("hello {}", name)
}

#[handler]
pub fn health_check() {
    // () -> 200 OK in poem
}

pub fn default_route(config: Configuration) -> Route {
    let mut route = Route::new()
        .at("/api/v1/hello/:name", get(hello))
        .at("/api/v1/health_check", get(health_check));

    let (api_service, ui) = self::routes::get_api_service(config);
    route = route.nest("/", api_service).nest("/docs", ui);

    route
}
