mod health_check;
mod subscribe;

use anyhow::{Context, Result};
use poem::test::TestClient;
use poem::Route;
use sea_orm::*;
use sea_orm_migration::prelude::*;
use tracing_test::traced_test;
use wiremock::MockServer;
use zero2prod::configuration::get_test_configuration;
use zero2prod::context::Context as RouteContext;
use zero2prod::routes::default_route;

pub struct TestApp {
    cli: TestClient<Route>,
    db: DatabaseConnection,
    email_server: MockServer,
}

#[traced_test]
pub async fn get_test_app() -> Result<TestApp> {
    let email_server = MockServer::start().await;
    let mut conf = get_test_configuration("config/test").expect("fail to get conf");
    conf.email_client.api_base_url = email_server.uri();
    let context = RouteContext::new(conf.clone()).await?;
    let db = context.db.clone();
    // migrate db
    let schema_manager = SchemaManager::new(&db);

    assert!(schema_manager
        .has_table("subscriptions")
        .await
        .context("fail to execute table existence check")?);

    let app = default_route(conf, context).await;
    let cli = TestClient::new(app);
    Ok(TestApp {
        cli,
        db,
        email_server,
    })
}
