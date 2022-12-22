use anyhow::Result;
use poem::http::StatusCode;
use wiremock::{matchers::path, Mock, ResponseTemplate};

use crate::api::helpers::get_test_app;
use crate::api::helpers::{delete_subscriber_by_id, email, get_first_link, post_subscription};

#[tokio::test]
async fn confirmations_without_token_rejected_with_400() -> Result<()> {
    let app = get_test_app().await?;
    let cli = &app.cli;
    let resp = cli.get("/subscriptions/confirm").send().await;
    resp.assert_status(StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn subscribe_and_then_confirm() -> Result<()> {
    let app = get_test_app().await?;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let data = format!("username=lin&email={}", email().to_string());
    let resp = post_subscription(&app.cli, data).await;
    resp.assert_status(StatusCode::OK);
    // let resp_json = resp.json().await;
    // later we use it to clean the user info
    // let id = resp_json.value().object().get("id").i64() as i32;

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
