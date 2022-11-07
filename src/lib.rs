use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
// use tracing_subscriber::fmt::format::FmtSpan;

pub mod configuration;
pub mod entities;
mod migrator;
pub mod routes;
mod startup;

pub use migrator::Migrator;
pub use startup::get_database_connection;

pub fn setup_logger(log_level: &str) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    let formatting_layer = BunyanFormattingLayer::new("zero2prod".into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);

    set_global_default(subscriber).expect("Fail to set up traincg subscriber");
    //let level = match log_level.to_ascii_lowercase().as_str() {
    //    "debug" => tracing::Level::DEBUG,
    //    "warn" => tracing::Level::WARN,
    //    _ => tracing::Level::INFO,
    //};

    //let stdout = std::io::stdout;

    //tracing_subscriber::fmt()
    //    .json()
    //    .with_current_span(true)
    //    .with_span_events(FmtSpan::FULL)
    //    .with_max_level(level)
    //    .with_ansi(false) // disable color output
    //    .with_line_number(level == tracing::Level::DEBUG)
    //    .with_writer(stdout) // both stdout and a file
    //    .init(); // since then, tracing::{info, warn, debug, trace} make effects
}
