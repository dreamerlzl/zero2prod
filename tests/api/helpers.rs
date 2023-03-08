use anyhow::{Context, Result};
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

pub fn get_first_link(text: &str, port: u16) -> String {
    let finder = LinkFinder::new();
    let links: Vec<_> = finder
        .links(text)
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
    assert_eq!(links.len(), 1);
    let raw_link = links[0].as_str().to_owned();
    let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
    confirmation_link
        .set_port(Some(port))
        .expect("fail to set port");
    confirmation_link.to_string()
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

pub struct TestAppWithCookie {
    pub cookie_cli: reqwest::Client,
    db: DatabaseConnection,
    pub test_user: TestUser,
    address: String,
    pub email_server: MockServer,
    port: u16,
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

    pub async fn post_logout(&self) -> Result<reqwest::Response, reqwest::Error> {
        self.cookie_cli
            .post(format!("{}/logout", &self.address))
            .send()
            .await
    }

    pub async fn get_login_html(&self) -> String {
        let resp = self
            .cookie_cli
            .get(format!("{}/login", self.address))
            .send()
            .await
            .expect("fail to get login page");
        resp.text().await.expect("fail to get resp in text")
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.cookie_cli
            .get(format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("fail to get admin dashboard in test")
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.cookie_cli
            .get(format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("failed to get /admin/password")
    }

    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.cookie_cli
            .post(format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("failed to post password change")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_newsletters<B>(&self, body: B) -> reqwest::Response
    where
        B: serde::Serialize,
    {
        self.cookie_cli
            .post(format!("{}/admin/newsletters", &self.address))
            .form(&body)
            .send()
            .await
            .expect("fail to send resp to post newsletters")
    }

    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter()
            .await
            .text()
            .await
            .expect("fail to get html from newsletters")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.cookie_cli
            .get(format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("fail to get")
    }

    pub async fn post_subscription<T: 'static + Into<reqwest::Body>>(
        &self,
        data: T,
    ) -> reqwest::Response {
        self.cookie_cli
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await
            .expect("fail to subscribe")
    }

    pub async fn confirm_subscription(&self, url: &str) -> Result<()> {
        let resp = self
            .cookie_cli
            .get(url)
            .send()
            .await
            .context(format!("fail to confirm subscriptions link: {}", url))?;
        assert_eq!(resp.status().as_u16(), reqwest::StatusCode::OK, "{}", url);
        Ok(())
    }

    pub fn get_confirmation_link(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let html = get_first_link(body["HtmlBody"].as_str().unwrap(), self.port);
        let html = reqwest::Url::parse(&html).unwrap();
        let text = get_first_link(body["TextBody"].as_str().unwrap(), self.port);
        let plain_text = reqwest::Url::parse(&text).unwrap();
        ConfirmationLinks { html, plain_text }
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
        port: app_port,
    })
}

pub fn assert_is_redirect_to(resp: &reqwest::Response, uri: &str) {
    assert_eq!(resp.status().as_u16(), 303, "{}", uri);
    assert_eq!(resp.headers().get::<&str>("Location").unwrap(), uri);
}
