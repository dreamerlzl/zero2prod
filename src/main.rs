use anyhow::Result;
use poem::{
    listener::TcpListener,
    session::{CookieConfig, RedisStorage, ServerSession},
    EndpointExt, Server,
};
use redis::{aio::ConnectionManager, Client};
use secrecy::ExposeSecret;
use tracing::info;
use zero2prod_api::{
    configuration::get_configuration, context::StateContext, routes::default_route, setup_logger,
};

#[tokio::main]
async fn main() -> Result<()> {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "DEBUG".into());
    setup_logger(&log_level);
    let conf = get_configuration().expect("fail to read configuration");

    let app_port = conf.app.port;
    let redis_uri = conf.redis_uri.expose_secret().clone();
    let context = StateContext::new(conf.clone()).await?;

    // set routing
    let route = default_route(conf, context.clone()).await;
    let client = Client::open(redis_uri)?;
    let app = route.with(ServerSession::new(
        CookieConfig::default(),
        RedisStorage::new(ConnectionManager::new(client).await?),
    ));

    // start the tcp listener
    let addr = format!("0.0.0.0:{}", app_port);
    info!(addr, "zero2prod listening on");
    Server::new(TcpListener::bind(addr)).run(app).await?;
    Ok(())
}
