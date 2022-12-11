mod health_check;
mod subscribe;

use anyhow::{Context, Result};
use poem::test::TestClient;
use poem::Route;
use sea_orm::*;
use sea_orm_migration::prelude::*;
use tracing_test::traced_test;
use zero2prod::configuration::get_test_configuration;
use zero2prod::context::Context as RouteContext;
use zero2prod::routes::default_route;
use zero2prod::Migrator;

#[traced_test]
pub async fn get_client_and_db() -> Result<(TestClient<Route>, DatabaseConnection)> {
    let conf = get_test_configuration("config/test").expect("fail to get conf");
    let context = RouteContext::new(conf.clone()).await?;
    let db = context.db.clone();
    // migrate db
    let schema_manager = SchemaManager::new(&db);
    Migrator::refresh(&db)
        .await
        .context("fail to migrate db for test")?;

    assert!(schema_manager
        .has_table("subscription")
        .await
        .context("fail to execute table existence check")?);

    let app = default_route(conf, context).await;
    Ok((TestClient::new(app), db))
}
