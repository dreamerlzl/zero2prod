use config::{Config, Environment};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct Configuration {
    pub app_port: u16,
    pub log_level: Option<String>,
    pub db: RelationalDBSettings,
}

#[derive(Deserialize, Debug)]
#[allow(unused)]
pub struct RelationalDBSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub name: String,
}

pub fn get_configuration() -> Result<Configuration, config::ConfigError> {
    let conf = Config::builder()
        .set_default("log_level", Some("DEBUG"))?
        .add_source(config::File::with_name("config/prod").required(false))
        .add_source(
            Environment::with_prefix("app")
                .try_parsing(true)
                .separator("__"),
        )
        .build()?;
    conf.try_deserialize()
}

pub fn get_test_configuration(path: &str) -> Result<Configuration, config::ConfigError> {
    let conf = Config::builder()
        .set_default("log_level", Some("DEBUG"))?
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
    use serial_test::{parallel, serial};

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
    #[parallel]
    #[ignore]
    fn test_env() {
        env::set_var("APP__APP_PORT", "8081");
        env::set_var("APP__DB__USERNAME", "foo");
        env::set_var("APP__DB__PASSWORD", "bar");
        env::set_var("APP__DB__PORT", "1234");
        env::set_var("APP__DB__HOST", "localhost");
        env::set_var("APP__DB__NAME", "test");

        let conf = get_test_configuration("config/test").expect("fail to get conf");

        assert_eq!(conf.db.username, "foo");
        assert_eq!(conf.db.password, "bar");
        assert_eq!(conf.db.port, 1234);
        assert_eq!(conf.db.host, "localhost");
        assert_eq!(conf.db.name, "test");
        assert_eq!(conf.app_port, 8081);

        env::remove_var("APP__APP_PORT");
        env::remove_var("APP__DB__USERNAME");
        env::remove_var("APP__DB__PASSWORD");
        env::remove_var("APP__DB__PORT");
        env::remove_var("APP__DB__HOST");
        env::remove_var("APP__DB__NAME");
    }

    // create a temporary configuration.yml file
    #[test]
    #[parallel]
    #[ignore]
    fn test_file() {
        // create a temporary configuration.yaml under $root/config/

        create_dir_all("config").expect("fail to create dir config");
        let path_str = format!("config/test-{}.yaml", get_random_str());
        let path = Path::new(&path_str);
        {
            let mut file = File::create(&path).expect("fail to create the test configuration yaml");
            let content = "app_port: 1234
db:
  username: foo
  password: 123
  host: bar
  port: 111
  name: test";
            file.write_all(content.as_bytes())
                .expect("fail to write config content");
        }

        let conf = get_test_configuration(&path_str).expect("fail to get conf");
        assert_eq!(conf.db.username, "foo");
        assert_eq!(conf.db.password, "123");
        assert_eq!(conf.db.port, 111);
        assert_eq!(conf.db.host, "bar");
        assert_eq!(conf.db.name, "test");
        assert_eq!(conf.app_port, 1234);

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
            let mut file = File::create(&path).expect("fail to create the test configuration yaml");
            let content = "app_port: 1234
db:
  username: foo
  host: bar
  name: test";
            file.write_all(content.as_bytes())
                .expect("fail to write config content");
            file.flush().expect("fail to flush files");
        }
        env::set_var("APP__DB__PASSWORD", "aaa");
        env::set_var("APP__DB__PORT", "111");

        let conf = get_test_configuration(&path_str).expect("fail to get conf");
        assert_eq!(conf.db.username, "foo");
        assert_eq!(conf.db.password, "aaa");
        assert_eq!(conf.db.port, 111);
        assert_eq!(conf.db.host, "bar");
        assert_eq!(conf.db.name, "test");
        assert_eq!(conf.app_port, 1234);
        remove_file(path).expect("fail to remove test config");

        env::remove_var("APP__DB__PASSWORD");
        env::remove_var("APP__DB__PORT");
    }
}
