use anyhow::Result;
use poem::http::StatusCode;
use sqlx::{Pool, Postgres};
use wiremock::{matchers::path, Mock, ResponseTemplate};

use crate::api::helpers::{email, get_first_link, get_test_app, post_subscription};

#[sqlx::test]
async fn confirmations_without_token_rejected_with_400(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    let cli = &app.cli;
    let resp = cli.get("/subscriptions/confirm").send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn subscribe_and_then_confirm(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let data = format!("username=lin&email={}", email().to_string());
    let resp = post_subscription(&app.cli, data).await;
    resp.assert_status(StatusCode::OK);

    let email_request = app.email_server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&email_request[0].body).unwrap();
    let confirm_link = get_first_link(&body["TextBody"].as_str().unwrap());
    let mut confirm_link = reqwest::Url::parse(&confirm_link)?;
    assert_eq!(confirm_link.host_str().unwrap(), "127.0.0.1");
    assert_eq!(confirm_link.path(), "/subscriptions/confirm");
    confirm_link.set_query(Some("token=123"));
    let resp = app.cli.get(confirm_link).send().await;
    resp.assert_status(StatusCode::OK);
    Ok(())
}
