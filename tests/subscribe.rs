use std::error::Error;

use poem::http::StatusCode;
use poem::test::TestClient;
use poem::Route;
use sea_orm::*;
use tracing::error;
use tracing_test::traced_test;
use zero2prod::configuration::get_test_configuration;
use zero2prod::entities::subscription;
use zero2prod::routes::default_route;

async fn get_client_and_db() -> (TestClient<Route>, DatabaseConnection) {
    let conf = get_test_configuration("config/test").expect("fail to get conf");
    let sql = &conf.db;
    let db_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        sql.username, sql.password, sql.host, sql.port, sql.name,
    );
    let db = Database::connect(db_url)
        .await
        .expect("fail to get sql db connection");
    let app = default_route(conf).await;
    (TestClient::new(app), db)
}

#[tokio::test]
#[traced_test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let (cli, db) = get_client_and_db().await;
    let valid_data = ["user=lzl&email=lzl2@gmail.com", "user=foo&email=bar@qq.com"];

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
}

#[tokio::test]
#[traced_test]
async fn subscribe_returns_400_for_invalid_data() {
    let (cli, _) = get_client_and_db().await;
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
}
