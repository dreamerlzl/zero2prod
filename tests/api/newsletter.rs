use anyhow::Result;
use poem::http::StatusCode;
use sqlx::{Pool, Postgres};
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use super::helpers::{
    get_confirmation_link, get_test_app, post_subscription, ConfirmationLinks, TestApp,
};

#[sqlx::test]
async fn newsletters_returns_400_given_invalid_data(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    let test_cases = vec![
        // missing content
        serde_json::json!({"title": "abc"}),
        // missing title
        serde_json::json!({"content": {"html":"a", "text":"b"}}),
    ];

    for test_case in test_cases {
        let resp = app.post_newsletters(test_case).await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

// submit newsletter to /newsletter via a json post request
// {
//  "title": "",
//  "content": {
//    "text": "text",
//    "html": "html"
//  }
// }
// the newsletter should not be delivered to unconfirmed subscribers
#[sqlx::test]
async fn newsletters_not_delivered_to_unconfirmed_subscribers(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    create_unconfirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;
    let request = serde_json::json!({
        "title": "title",
        "content": {
            "text": "plain text",
            "html": "<p>html body</p>",
        },
    });
    let resp = app.post_newsletters(request).await;
    resp.assert_status(StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn newsletters_delivered_to_confirmed_subscribers(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let request = serde_json::json!({
        "title": "title",
        "content": {
            "text": "plain text",
            "html": "<p>html body</p>",
        },
    });
    let resp = app.post_newsletters(request).await;
    resp.assert_status(StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn requests_without_authorization_are_rejected(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    let body = serde_json::json!({
        "title": "title",
        "content": {
            "text": "plain text",
            "html": "<p>html body</p>",
        },
    });
    let resp = app.post_newsletters_without_auth(body).await;
    resp.assert_status(StatusCode::UNAUTHORIZED);
    resp.assert_header("WWW-Authenticate", r#"Basic realm="publish""#);
    Ok(())
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    // mount_as_scoped -> the mock would no longer work after _guard is dropped
    let _guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    let resp = post_subscription(&app.cli, "username=lzl&email=lzl2@gmail.com").await;
    resp.assert_status(StatusCode::OK);
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    get_confirmation_link(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await.html;
    // confirmation_link.set_port(Some());
    app.cli
        .get(confirmation_link)
        .send()
        .await
        .assert_status(StatusCode::OK);
}
