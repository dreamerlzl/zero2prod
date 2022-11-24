use anyhow::Result;
use poem::{listener::TcpListener, Server};
use tracing::info;
use zero2prod::configuration::get_configuration;
use zero2prod::context::Context;
use zero2prod::routes::default_route;
use zero2prod::setup_logger;

#[tokio::main]
async fn main() -> Result<()> {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "DEBUG".into());
    setup_logger(&log_level);
    let conf = get_configuration().expect("fail to read configuration");

    let app_port = conf.app_port;
    let context = Context::new(conf.clone()).await?;

    // set routing
    let route = default_route(conf, context).await;

    // start the tcp listener
    let addr = format!("0.0.0.0:{}", app_port);
    info!(addr, "zero2prod listening on");
    Server::new(TcpListener::bind(addr)).run(route).await?;
    Ok(())
}
