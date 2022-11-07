use poem::{listener::TcpListener, Server};
use tracing::info;
use zero2prod::configuration::get_configuration;
use zero2prod::routes::default_route;
use zero2prod::setup_logger;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let conf = get_configuration().expect("fail to read configuration");
    setup_logger(conf.log_level.as_ref().unwrap());

    let app_port = conf.app_port;
    let route = default_route(conf).await;
    let addr = format!("127.0.0.1:{}", app_port);
    info!(addr, "zero2prod listening on");
    Server::new(TcpListener::bind(addr)).run(route).await
}
