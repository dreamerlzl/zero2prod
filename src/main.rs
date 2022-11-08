use anyhow::Result;
use poem::{listener::TcpListener, Server};
use tracing::info;
use zero2prod::configuration::get_configuration;
use zero2prod::get_database_connection;
use zero2prod::routes::default_route;
use zero2prod::setup_logger;

#[tokio::main]
async fn main() -> Result<()> {
    let conf = get_configuration().expect("fail to read configuration");
    setup_logger(conf.log_level.as_ref().unwrap());

    let app_port = conf.app_port;
    let db = get_database_connection(&conf).await?;

    // set routing
    let route = default_route(conf, db).await;

    // start the tcp listener
    let addr = format!("127.0.0.1:{}", app_port);
    info!(addr, "zero2prod listening on");
    Server::new(TcpListener::bind(addr)).run(route).await?;
    Ok(())
}
