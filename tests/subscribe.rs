use std::error::Error;

use anyhow::{Context, Result};
use poem::http::StatusCode;
use poem::test::TestClient;
use poem::Route;
use sea_orm::*;
use sea_orm_migration::prelude::*;
use serial_test::serial;
use tracing::error;
use tracing_test::traced_test;
use zero2prod::configuration::get_test_configuration;
use zero2prod::entities::subscription;
use zero2prod::get_database_connection;
use zero2prod::routes::default_route;
use zero2prod::Migrator;

#[traced_test]
async fn get_client_and_db() -> Result<(TestClient<Route>, DatabaseConnection)> {
    let conf = get_test_configuration("config/test").expect("fail to get conf");
    let db = get_database_connection(&conf)
        .await
        .context("fail to get db conn")?;

    // migrate db
    let schema_manager = SchemaManager::new(&db);
    Migrator::refresh(&db)
        .await
        .context("fail to migrate db for test")?;

    assert!(schema_manager
        .has_table("subscription")
        .await
        .context("fail to execute table existence check")?);

    let app = default_route(conf, db.clone()).await;
    Ok((TestClient::new(app), db))
}

#[tokio::test]
#[traced_test]
#[serial]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    let (cli, db) = get_client_and_db().await?;
    let valid_data = [
        "username=lzl&email=lzl2@gmail.com",
        "username=foo&email=bar@qq.com",
    ];

    for data in valid_data.into_iter() {
        let resp = cli
            .post("/subscription")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await;
        resp.assert_status(StatusCode::OK);
        let resp_json = resp.json().await;
        let id = resp_json.value().object().get("id").i64() as i32;
        let new_user = subscription::ActiveModel {
            id: ActiveValue::Set(id),
            ..Default::default()
        };
        if let Err(e) = new_user.delete(&db).await {
            error!(error = e.source(), id = id, "fail to delete test data");
        }
    }
    Ok(())
}

#[tokio::test]
#[traced_test]
#[serial]
async fn subscribe_returns_400_for_invalid_data() -> Result<()> {
    let (cli, _) = get_client_and_db().await?;
    let invalid_data = ["", "name=lzl", "email=aaa", "name=lzl&email=aaa", "foobar"];

    for data in invalid_data.into_iter() {
        let resp = cli
            .post("/subscription")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
    Ok(())
}
