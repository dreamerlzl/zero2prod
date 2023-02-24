use std::path::Path;

use config::{Config, Environment};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use tracing::info;

use crate::domain::Email;

#[derive(Deserialize, Debug, Clone)]
#[allow(unused)]
pub struct Configuration {
    pub app: AppSettings,
    pub db: RelationalDBSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: Secret<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(unused)]
pub struct AppSettings {
    pub port: u16,
    pub base_url: String,
    pub admin_username: String,
    pub admin_password: String,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(unused)]
pub struct RelationalDBSettings {
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub name: String,
    pub require_ssl: bool,
}

impl RelationalDBSettings {
    // these two methods are only for pg
    pub fn options_without_db(&self) -> PgConnectOptions {
        let mut options = PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port);
        if self.require_ssl {
            options = options.ssl_mode(PgSslMode::Require);
        }
        options
    }

    pub fn options_with_db(&self) -> PgConnectOptions {
        self.options_without_db().database(&self.name)
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct EmailClientSettings {
    pub api_base_url: String,
    pub sender_email: String,
    pub authorization_token: String,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<Email, String> {
        Email::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    let environment = std::env::var("APP__ENVIRONMENT").unwrap_or_else(|_| "test".to_owned());
    info!("using environment: {}", environment);
    let conf_path = Path::new("config").join(environment);
    let default_conf_path = Path::new("config").join("default");

    let conf = Config::builder()
        .add_source(
            config::File::with_name(default_conf_path.as_path().to_str().unwrap()).required(false),
        )
        .add_source(config::File::with_name(conf_path.as_path().to_str().unwrap()).required(false))
        .add_source(
            Environment::with_prefix("app")
                .try_parsing(true)
                .separator("__"),
        )
        .build()?;
    conf.try_deserialize()
}

pub fn get_test_configuration(path: &str) -> Result<Configuration, config::ConfigError> {
    let default_conf_path = Path::new("config").join("default");
    let conf = Config::builder()
        .add_source(
            config::File::with_name(default_conf_path.as_path().to_str().unwrap()).required(false),
        )
        .add_source(config::File::with_name(path).required(false))
        .add_source(
            Environment::with_prefix("app")
                .try_parsing(true)
                .separator("__"),
        )
        .build()?;
    conf.try_deserialize()
}

#[cfg(test)]
mod tests {
    use std::fs::{create_dir_all, remove_file, File};
    use std::{env, io::Write, path::Path};

    use rand::{distributions::Alphanumeric, Rng};
    use secrecy::ExposeSecret;
    use serial_test::serial;

    use super::get_test_configuration;

    fn get_random_str() -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect()
    }

    // just test Environment variables
    #[test]
    #[serial]
    #[ignore]
    fn test_env() {
        env::set_var("APP__APP_PORT", "8081");
        env::set_var("APP__DB__USERNAME", "foo");
        env::set_var("APP__DB__PASSWORD", "bar");
        env::set_var("APP__DB__PORT", "1234");
        env::set_var("APP__DB__HOST", "localhost");
        env::set_var("APP__DB__NAME", "test");
        env::set_var("APP__DB__REQUIRE_SSL", "TRUE");

        let conf = get_test_configuration("config/test").expect("fail to get conf");

        assert_eq!(conf.db.username, "foo");
        assert_eq!(conf.db.password.expose_secret(), "bar");
        assert_eq!(conf.db.port, 1234);
        assert_eq!(conf.db.host, "localhost");
        assert_eq!(conf.db.name, "test");
        assert_eq!(conf.app.port, 8081);
        assert!(conf.db.require_ssl);

        env::remove_var("APP__APP_PORT");
        env::remove_var("APP__DB__USERNAME");
        env::remove_var("APP__DB__PASSWORD");
        env::remove_var("APP__DB__PORT");
        env::remove_var("APP__DB__HOST");
        env::remove_var("APP__DB__NAME");
        env::remove_var("APP__DB__REQUIRE_SSL");
    }

    // create a temporary configuration.yml file
    #[test]
    #[serial]
    #[ignore]
    fn test_file() {
        // create a temporary configuration.yaml under $root/config/

        create_dir_all("config").expect("fail to create dir config");
        let path_str = format!("config/test-{}.yaml", get_random_str());
        let path = Path::new(&path_str);
        {
            let mut file = File::create(path).expect("fail to create the test configuration yaml");
            let content = "
app:
  port: 1234
  base_url: 'abc'
email_client:
  api_base_url: \"https://api.postmarkapp.com\"
  sender_email: \"something@gmail.com\"
  timeout_milliseconds: 10
db:
  username: foo
  password: 123
  host: bar
  port: 111
  require_ssl: true
  name: test";
            file.write_all(content.as_bytes())
                .expect("fail to write config content");
        }

        let conf = get_test_configuration(&path_str).expect("fail to get conf");
        assert_eq!(conf.db.username, "foo");
        assert_eq!(conf.db.password.expose_secret(), "123");
        assert_eq!(conf.db.port, 111);
        assert_eq!(conf.db.host, "bar");
        assert_eq!(conf.db.name, "test");
        assert_eq!(conf.app.port, 1234);
        assert!(conf.db.require_ssl);
        assert_eq!(conf.email_client.sender_email, "something@gmail.com");
        assert_eq!(
            conf.email_client.api_base_url,
            "https://api.postmarkapp.com"
        );

        remove_file(path).expect("fail to remove test config");
    }

    #[test]
    #[serial]
    #[ignore]
    fn test_hierarchy() {
        use std::fs::{create_dir_all, File};
        // create a temporary configuration.yaml under $root/config/

        create_dir_all("config").expect("fail to create dir config");
        let path_str = format!("config/test-{}.yaml", get_random_str());
        let path = Path::new(&path_str);
        {
            let mut file = File::create(path).expect("fail to create the test configuration yaml");
            let content = "
app:
  port: 1234
  base_url: 'http://127.0.0.1'
email_client:
  api_base_url: https://example.com
  sender_email: buffoonlzl0@gmail.com
  authorization_token: 123
  timeout_milliseconds: 100
db:
  username: foo
  host: bar
  require_ssl: true
  name: test";
            file.write_all(content.as_bytes())
                .expect("fail to write config content");
            file.flush().expect("fail to flush files");
        }
        env::set_var("APP__DB__PASSWORD", "aaa");
        env::set_var("APP__DB__PORT", "111");
        env::set_var("APP__DB__REQUIRE_SSL", "FALSE");

        let conf = get_test_configuration(&path_str).expect("fail to get conf");
        assert_eq!(conf.db.username, "foo");
        assert_eq!(conf.db.password.expose_secret(), "aaa");
        assert_eq!(conf.db.port, 111);
        assert_eq!(conf.db.host, "bar");
        assert_eq!(conf.db.name, "test");
        assert_eq!(conf.app.port, 1234);
        assert!(conf.db.require_ssl);
        remove_file(path).expect("fail to remove test config");

        env::remove_var("APP__DB__PASSWORD");
        env::remove_var("APP__DB__PORT");
    }
}
