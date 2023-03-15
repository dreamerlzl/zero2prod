use std::fmt::{Debug, Display};

use anyhow::Result;
use poem::{
    listener::TcpListener,
    session::{CookieConfig, RedisStorage, ServerSession},
    EndpointExt, Server,
};
use redis::{aio::ConnectionManager, Client};
use secrecy::ExposeSecret;
use tokio::task::JoinError;
use tracing::info;
use zero2prod_api::{
    configuration::get_configuration, context::StateContext,
    issue_delivery_worker::run_worker_until_stop, routes::default_route, setup_logger,
};

#[tokio::main]
async fn main() -> Result<()> {
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "DEBUG".into());
    setup_logger(&log_level);
    let conf = get_configuration().expect("fail to read configuration");

    let app_port = conf.app.port;
    let redis_uri = conf.redis_uri.expose_secret().clone();
    let context = StateContext::new(conf.clone()).await?;

    let worker = tokio::spawn(run_worker_until_stop(conf.clone()));
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
    let server = tokio::spawn(Server::new(TcpListener::bind(addr)).run(app));
    tokio::select! {
        o = server => {report_exit("api", o)},
        o = worker => {report_exit("worker", o)},
    }
    Ok(())
}

fn report_exit(task_name: &str, outcome: Result<Result<(), impl Debug + Display>, JoinError>) {
    match outcome {
        Ok(Ok(())) => {
            tracing::info!("{} has exited", task_name)
        }
        Ok(Err(e)) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{} failed",
                task_name
            )
        }
        Err(e) => {
            tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "{}' task failed to complete",
                task_name
            )
        }
    }
}
