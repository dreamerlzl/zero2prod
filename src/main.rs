use poem::{listener::TcpListener, Server};

use zero2prod::{configuration::get_configuration, default_route};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let conf = get_configuration().expect("fail to read configuration");
    let app_port = conf.app_port;
    let route = default_route(conf).await;
    Server::new(TcpListener::bind(format!("127.0.0.1:{}", app_port)))
        .run(route)
        .await
}
