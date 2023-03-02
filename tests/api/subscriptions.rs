use std::str::FromStr;

use anyhow::Result;
use poem::http::StatusCode;
use sea_orm::{prelude::Uuid, *};
use sqlx::{Pool, Postgres};
use wiremock::{matchers::path, Mock, ResponseTemplate};
use zero2prod_api::{entities::subscriptions, routes::subscriptions::ConfirmStatus};

use crate::{
    api::helpers::{email, get_first_link, get_test_app, post_subscription},
    normal_test,
};

#[sqlx::test]
async fn subscribe_returns_a_200_for_valid_form_data(pool: Pool<Postgres>) -> Result<()> {
    let test_app = get_test_app(pool).await?;
    let cli = &test_app.cli;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    let valid_data = [
        "username=lzl&email=lzl2@gmail.com",
        "username=foo&email=bar@qq.com",
    ];

    for data in valid_data.into_iter() {
        let resp = post_subscription(cli, data).await;
        resp.assert_status(StatusCode::OK);
    }
    Ok(())
}

#[sqlx::test]
async fn subscribe_persists_the_new_subscriber(pool: Pool<Postgres>) -> Result<()> {
    let test_app = get_test_app(pool).await?;
    let cli = &test_app.cli;
    let db = &test_app.db;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;
    let data = "username=lzl&email=bar@qq.com";
    let resp = post_subscription(cli, data).await;
    let resp_json = resp.json().await;
    let id = resp_json.value().object().get("id").string();
    let new_user = subscriptions::Entity::find_by_id(Uuid::from_str(id).unwrap())
        .one(db)
        .await?;
    assert!(new_user.is_some());
    let new_user = new_user.unwrap();
    assert_eq!(new_user.email, "bar@qq.com");
    assert_eq!(new_user.name, "lzl");
    assert_eq!(new_user.status, ConfirmStatus::Pending.to_string());
    Ok(())
}

normal_test!(subscribe_returns_400_for_invalid_data, [app] {
    let invalid_data = [
        "",
        "username=lzl",
        "email=aaa",
        "username=lzl&email=aaa",
        "foobar",
    ];

    for data in invalid_data.into_iter() {
        let resp = post_subscription(&app.cli, data).await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
});

#[sqlx::test]
async fn subscribe_returns_a_confirmation_email(pool: Pool<Postgres>) -> Result<()> {
    let test_app = get_test_app(pool).await?;

    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let cli = &test_app.cli;
    let data = format!("username=lin&email={}", email().to_string());
    let resp = post_subscription(cli, data).await;
    resp.assert_status(StatusCode::OK);
    let email_request = test_app.email_server.received_requests().await.unwrap();

    let body: serde_json::Value = serde_json::from_slice(&email_request[0].body).unwrap();
    // the port here doesn't matter
    let html_link = get_first_link(body["HtmlBody"].as_str().unwrap(), 7070);
    let text_link = get_first_link(body["TextBody"].as_str().unwrap(), 7070);
    assert_eq!(html_link, text_link);

    Ok(())
}
