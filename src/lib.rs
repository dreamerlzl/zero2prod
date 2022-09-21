pub mod configuration;
pub mod entities;
pub mod routes;

pub fn setup_logger(log_level: &str) {
    let level = match log_level.to_ascii_lowercase().as_str() {
        "debug" => tracing::Level::DEBUG,
        "warn" => tracing::Level::WARN,
        _ => tracing::Level::INFO,
    };

    let stdout = std::io::stdout;

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false) // disable color output
        .with_line_number(level == tracing::Level::DEBUG)
        .with_writer(stdout) // both stdout and a file
        .init(); // since then, tracing::{info, warn, debug, trace} make effects
}
