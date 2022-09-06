use std::convert::TryInto;

use config::Config;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Configuration {
    pub db: DatabaseSettings,
    pub app_port: u16,
}

#[derive(Deserialize)]
struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub db_name: String,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    let conf =Config::builder().add_source(config::File::with_name("configuration")).build()?;
    conf.try_deserialize()
}
