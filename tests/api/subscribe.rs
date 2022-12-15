use std::error::Error;

use anyhow::Result;
use fake::{faker::internet::en::SafeEmail, Fake};
use poem::{
    http::StatusCode,
    test::{TestClient, TestResponse},
    Body as PoemBody, Route,
};
use sea_orm::*;
use serial_test::serial;
use tracing::error;
use tracing_test::traced_test;
use wiremock::Mock;
use wiremock::{matchers::path, ResponseTemplate};
use zero2prod::domain::Email;
use zero2prod::entities::subscriptions;

use crate::api::get_test_app;

#[tokio::test]
#[traced_test]
#[serial]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    let test_app = get_test_app().await?;
    let cli = test_app.cli;
    let db = test_app.db;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    let valid_data = [
        "username=lzl&email=lzl2@gmail.com",
        "username=foo&email=bar@qq.com",
    ];

    for data in valid_data.into_iter() {
        let resp = post_subscription(&cli, data).await;
        resp.assert_status(StatusCode::OK);
        let resp_json = resp.json().await;
        let id = resp_json.value().object().get("id").i64() as i32;
        let new_user = subscriptions::ActiveModel {
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
    let test_app = get_test_app().await?;
    let cli = test_app.cli;
    let invalid_data = [
        "",
        "username=lzl",
        "email=aaa",
        "username=lzl&email=aaa",
        "foobar",
    ];

    for data in invalid_data.into_iter() {
        let resp = post_subscription(&cli, data).await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_confirmation_email() -> Result<()> {
    let test_app = get_test_app().await?;

    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let db = test_app.db;
    let cli = test_app.cli;
    let data = format!("username=lin&email={}", email().to_string());
    let resp = post_subscription(&cli, data).await;
    resp.assert_status(StatusCode::OK);
    let resp_json = resp.json().await;
    let id = resp_json.value().object().get("id").i64() as i32;
    let new_user = subscriptions::ActiveModel {
        id: ActiveValue::Set(id),
        ..Default::default()
    };
    if let Err(e) = new_user.delete(&db).await {
        error!(error = e.source(), id = id, "fail to delete test data");
    }

    Ok(())
}

async fn post_subscription<T: 'static + Into<PoemBody>>(
    cli: &TestClient<Route>,
    data: T,
) -> TestResponse {
    cli.post("/subscription")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(data)
        .send()
        .await
}

fn email() -> Email {
    Email::parse(SafeEmail().fake::<String>()).unwrap()
}
