use std::sync::Arc;

use anyhow::{Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use fake::{faker::internet::en::SafeEmail, Fake};
use linkify::LinkFinder;
use migration::{Migrator, MigratorTrait};
use once_cell::sync::Lazy;
use poem::test::{TestClient, TestResponse};
use poem::{Body, Route};
use sea_orm::prelude::Uuid;
use sea_orm::DatabaseConnection;
use sea_orm::*;
use sea_orm_migration::prelude::*;
use sqlx::{Pool, Postgres};
use wiremock::MockServer;
use zero2prod_api::configuration::get_test_configuration;
use zero2prod_api::context::StateContext;
use zero2prod_api::domain::Email;
use zero2prod_api::entities::user::{self, Entity as Users};
use zero2prod_api::routes::default_route;
use zero2prod_api::routes::newsletters::get_hash;
use zero2prod_api::setup_logger;

pub async fn post_subscription<T: 'static + Into<Body>>(
    cli: &TestClient<Route>,
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

pub struct TestApp {
    pub cli: TestClient<Route>,
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

    let context = Arc::new(context);
    let app = default_route(conf, context).await;
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
        self.cli
            .post("/newsletters")
            .body_json(&body)
            .header(
                "Authorization",
                format!(
                    "Basic {}",
                    general_purpose::STANDARD.encode(format!(
                        "{}:{}",
                        self.test_user.username, self.test_user.password,
                    ))
                ),
            )
            .send()
            .await
    }
}

pub struct TestUser {
    username: String,
    password: String,
    salt: String,
}

impl TestUser {
    fn generate() -> Self {
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
    let password_hash =
        get_hash(&test_user.password, &test_user.salt).context("fail to register_test_user")?;
    let new_user = user::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_name: ActiveValue::Set(test_user.username.clone()),
        password_hashed: ActiveValue::Set(password_hash),
        salt: ActiveValue::Set(test_user.salt.clone()),
    };
    Users::insert(new_user).exec(db).await?;
    Ok(())
}
