use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use fake::{faker::internet::en::SafeEmail, Fake};
use linkify::LinkFinder;
use migration::{Migrator, MigratorTrait};
use once_cell::sync::Lazy;
use poem::{
    listener::TcpListener,
    middleware::CookieJarManagerEndpoint,
    session::{CookieConfig, RedisStorage, ServerSession, ServerSessionEndpoint},
    test::{TestClient, TestResponse},
    Body, EndpointExt, Route, Server,
};
use redis::{aio::ConnectionManager, Client};
use sea_orm::{prelude::Uuid, DatabaseConnection, *};
use sea_orm_migration::prelude::*;
use secrecy::ExposeSecret;
use sqlx::{Pool, Postgres};
use wiremock::MockServer;
use zero2prod_api::{
    configuration::get_test_configuration, context::StateContext, domain::Email,
    routes::default_route, setup_logger,
};

pub async fn post_subscription<T: 'static + Into<Body>>(
    cli: &TestClient<ClientType>,
    data: T,
) -> TestResponse {
    cli.post("/subscriptions")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(data)
        .send()
        .await
}

pub fn email() -> Email {
    Email::parse(SafeEmail().fake::<String>()).unwrap()
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

pub fn get_confirmation_link(email_request: &wiremock::Request) -> ConfirmationLinks {
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    let html = get_first_link(body["HtmlBody"].as_str().unwrap());
    let html = reqwest::Url::parse(&html).unwrap();
    let text = get_first_link(body["TextBody"].as_str().unwrap());
    let plain_text = reqwest::Url::parse(&text).unwrap();
    ConfirmationLinks { html, plain_text }
}

pub fn get_first_link(text: &str) -> String {
    let finder = LinkFinder::new();
    let links: Vec<_> = finder
        .links(text)
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
    assert_eq!(links.len(), 1);
    links[0].as_str().to_owned()
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info";
    if std::env::var("TEST_LOG").is_ok() {
        setup_logger(default_filter_level);
    }
});

type ClientType =
    CookieJarManagerEndpoint<ServerSessionEndpoint<RedisStorage<ConnectionManager>, Route>>;

pub struct TestApp {
    pub cli: TestClient<ClientType>,
    pub db: DatabaseConnection,
    pub email_server: MockServer,
    pub test_user: TestUser,
}

pub async fn get_test_app(pool: Pool<Postgres>) -> Result<TestApp> {
    Lazy::force(&TRACING);
    let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
    Migrator::refresh(&db).await?;
    let email_server = MockServer::start().await;
    let mut conf = get_test_configuration("config/test").expect("fail to get conf");
    conf.email_client.api_base_url = email_server.uri();
    let mut context = StateContext::new(conf.clone()).await?;
    context.db = db.clone();

    let client = Client::open(conf.redis_uri.expose_secret().clone())?;
    let app = default_route(conf, context).await.with(ServerSession::new(
        CookieConfig::default(),
        RedisStorage::new(ConnectionManager::new(client).await?),
    ));
    let cli = TestClient::new(app);
    let test_user = TestUser::generate();
    register_test_user(&db, &test_user).await?;
    Ok(TestApp {
        cli,
        db,
        email_server,
        test_user,
    })
}

impl TestApp {
    pub async fn post_newsletters_without_auth(&self, body: serde_json::Value) -> TestResponse {
        self.cli.post("/newsletters").body_json(&body).send().await
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> TestResponse {
        post_newsletters(&self.cli, &self.test_user, body).await
    }
}

#[derive(Clone)]
pub struct TestUser {
    pub username: String,
    pub password: String,
    salt: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            salt: Uuid::new_v4().to_string(),
        }
    }
}

pub async fn register_test_user(
    db: &DatabaseConnection,
    test_user: &TestUser,
) -> anyhow::Result<()> {
    zero2prod_api::auth::register_test_user(
        db,
        &test_user.username,
        &test_user.password,
        &test_user.salt,
    )
    .await
}

pub async fn post_newsletters(
    cli: &TestClient<ClientType>,
    test_user: &TestUser,
    body: serde_json::Value,
) -> TestResponse {
    cli.post("/newsletters")
        .body_json(&body)
        .header(
            "Authorization",
            format!(
                "Basic {}",
                general_purpose::STANDARD
                    .encode(format!("{}:{}", test_user.username, test_user.password))
            ),
        )
        .send()
        .await
}

pub struct TestAppWithCookie {
    cookie_cli: reqwest::Client,
    db: DatabaseConnection,
    pub test_user: TestUser,
    address: String,
    email_server: MockServer,
}

impl TestAppWithCookie {
    pub async fn post_login<Body>(&self, body: &Body) -> Result<reqwest::Response, reqwest::Error>
    where
        Body: serde::Serialize,
    {
        self.cookie_cli
            .post(format!("{}/login", self.address))
            .form(body)
            .send()
            .await
    }

    pub async fn get_login_html(&self) -> Result<String, reqwest::Error> {
        let resp = self
            .cookie_cli
            .get(format!("{}/login", self.address))
            .send()
            .await?;
        resp.text().await
    }

    pub async fn get_admin_dashboard(&self) -> Result<reqwest::Response, reqwest::Error> {
        self.cookie_cli
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
    }
}

pub async fn get_test_app_with_cookie(pool: Pool<Postgres>) -> Result<TestAppWithCookie> {
    Lazy::force(&TRACING);
    let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
    Migrator::refresh(&db).await?;
    let email_server = MockServer::start().await;
    let mut conf = get_test_configuration("config/test").expect("fail to get conf");
    conf.email_client.api_base_url = email_server.uri();
    let mut context = StateContext::new(conf.clone()).await?;
    context.db = db.clone();

    let client = Client::open(conf.redis_uri.expose_secret().clone())?;
    let app = default_route(conf, context).await.with(ServerSession::new(
        CookieConfig::default(),
        RedisStorage::new(ConnectionManager::new(client).await?),
    ));
    let app_port = std::net::TcpListener::bind(format!("127.0.0.1:{}", 0))?
        .local_addr()
        .context("fail to get local addr")?
        .port();
    let addr = format!("127.0.0.1:{}", app_port);
    tokio::spawn(async move {
        Server::new(TcpListener::bind(addr))
            .run(app)
            .await
            .expect("fail to create a new test server");
    });

    let test_user = TestUser::generate();
    register_test_user(&db, &test_user)
        .await
        .context("fail to register test user")?;
    let cookie_cli = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()?;
    Ok(TestAppWithCookie {
        address: format!("http://127.0.0.1:{}", app_port),
        cookie_cli,
        db,
        email_server,
        test_user,
    })
}

pub fn assert_is_redirect_to(resp: &reqwest::Response, uri: &str) {
    assert_eq!(resp.status().as_u16(), 303);
    assert_eq!(resp.headers().get::<&str>("Location").unwrap(), uri);
}
