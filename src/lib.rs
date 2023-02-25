use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
// use tracing_subscriber::fmt::format::FmtSpan;

pub mod auth;
pub mod configuration;
pub mod context;
pub mod domain;
pub mod email_client;
pub mod entities;
pub mod routes;
pub mod session_state;
mod startup;
pub mod utils;

pub use startup::{get_database_connection, get_email_client};

pub fn setup_logger(log_level: &str) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    set_global_default(subscriber).expect("Fail to set up traincg subscriber");
}
