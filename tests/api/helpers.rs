use std::sync::Arc;

use anyhow::Result;
use fake::{faker::internet::en::SafeEmail, Fake};
use linkify::LinkFinder;
use migration::{Migrator, MigratorTrait};
use once_cell::sync::Lazy;
use poem::test::{TestClient, TestResponse};
use poem::{Body, Route};
use sea_orm::DatabaseConnection;
use sea_orm::*;
use sea_orm_migration::prelude::*;
use sqlx::{Pool, Postgres};
use wiremock::MockServer;
use zero2prod_api::configuration::get_test_configuration;
use zero2prod_api::context::StateContext;
use zero2prod_api::domain::Email;
use zero2prod_api::routes::default_route;
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
    Ok(TestApp {
        cli,
        db,
        email_server,
    })
}
