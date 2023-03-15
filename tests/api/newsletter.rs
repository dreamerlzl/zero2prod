use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use uuid::Uuid;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use super::helpers::{assert_is_redirect_to, ConfirmationLinks, TestAppWithCookie};
use crate::{cookie_test, login_test};

//fn when_sending_an_email() -> MockBuilder {
//    Mock::given(path("/email")).and(method("POST"))
//}

//login_test!(transient_error_do_not_cause_duplicate_deliveries_on_retries, [app]{
//    create_confirmed_subscriber(&app).await;
//    create_confirmed_subscriber(&app).await;
//    when_sending_an_email()
//        .respond_with(ResponseTemplate::new(200))
//        .up_to_n_times(1)
//        .expect(1)
//        .mount(&app.email_server)
//        .await;
//    when_sending_an_email()
//        .respond_with(ResponseTemplate::new(500))
//        .up_to_n_times(1)
//        .expect(1)
//        .mount(&app.email_server)
//        .await;
//    let body = serde_json::json!({
//        "title": "title",
//        "text_content": "plain text",
//        "html_content": "<p>html body</p>",
//        "idempotency_key": Uuid::new_v4().to_string(),
//    });
//    let resp = app.post_newsletters(body.clone()).await;
//    assert_eq!(resp.status().as_u16(), 500);
//
//    // retry submitting the form
//    when_sending_an_email()
//        .respond_with(ResponseTemplate::new(200))
//        .expect(1)
//        .named("Delivery retry")
//        .mount(&app.email_server)
//        .await;
//    let resp = app.post_newsletters(body).await;
//    assert_eq!(resp.status().as_u16(), 303);
//});

login_test!(concurrent_form_submission_handled_gracefully, [app]{
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(
            // the delay duration here must be within the timeout conf for email client
            // otherwise, neither request would make it
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(2)))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let body = serde_json::json!({
        "title": "title",
        "text_content": "plain text",
        "html_content": "<p>html body</p>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let resp1 = app.post_newsletters(body.clone());
    let resp2 = app.post_newsletters(body);
    let (resp1, resp2) = tokio::join!(resp1, resp2);
    assert_eq!(resp1.status(), resp2.status());
    assert_eq!(resp1.text().await.expect("fail to get text body from resp1"), resp2.text().await.expect("fail to get text body from resp2"));
    app.dispatch_all_pending_emails().await;
});

login_test!(newsletters_returns_400_given_invalid_data, [app] {
    let test_cases = vec![
        // missing content
        serde_json::json!({"title": "abc"}),
        // missing title
        serde_json::json!({
            "text_content": "a",
            "html_content": "b",
        }),
    ];

    for test_case in test_cases {
        let resp = app.post_newsletters(test_case).await;
        assert_eq!(resp.status().as_u16(), reqwest::StatusCode::BAD_REQUEST);
    }
});

// submit newsletter to /newsletter via a json post request
// {
//  "title": "",
//  "content": {
//    "text": "text",
//    "html": "html"
//  }
// }
// the newsletter should not be delivered to unconfirmed subscribers
login_test!(newsletters_not_delivered_to_unconfirmed_subscribers, [app]{
    create_unconfirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;
    let request = serde_json::json!({
        "title": "title",
        "text_content": "plain text",
        "html_content": "<p>html body</p>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let resp = app.post_newsletters(request).await;
    assert_is_redirect_to(&resp, "/admin/newsletters");
    app.dispatch_all_pending_emails().await;
});

login_test!(newsletters_delivered_to_confirmed_subscribers, [app]{
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let request = serde_json::json!({
        "title": "title",
        "text_content": "plain text",
        "html_content": "<p>html body</p>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let resp = app.post_newsletters(request).await;
    assert_is_redirect_to(&resp, "/admin/newsletters");
    app.dispatch_all_pending_emails().await;
});

login_test!(newsletter_creation_is_idempotent, [app]{
    create_confirmed_subscriber(&app).await;
    Mock::given(path("/email"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;
    let request = serde_json::json!({
        "title": "title",
        "text_content": "plain text",
        "html_content": "<p>html body</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string(),
    });
    let resp = app.post_newsletters(request.clone()).await;
    assert_is_redirect_to(&resp, "/admin/newsletters");

    // part2
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter issue has been published!"));
    let resp = app.post_newsletters(request).await;
    assert_is_redirect_to(&resp, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter issue has been published!"));
    app.dispatch_all_pending_emails().await;
});

cookie_test!(requests_without_authorization_are_redirected, [app]{
    let body = serde_json::json!({
        "title": "title",
        "text_content": "plain text",
        "html_content": "<p>html body</p>",
        "idempotency_key": Uuid::new_v4().to_string(),
    });
    let resp = app.post_newsletters(body).await;
    assert_is_redirect_to(&resp, "/login");
});

async fn create_unconfirmed_subscriber(app: &TestAppWithCookie) -> ConfirmationLinks {
    // mount_as_scoped -> the mock would no longer work after _guard is dropped
    let _guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;
    let username: String = Name().fake();
    let email: String = SafeEmail().fake();
    let resp = app
        .post_subscription(
            serde_urlencoded::to_string(&serde_json::json!({
                "username": username,
                "email": email,
            }))
            .unwrap(),
        )
        .await;
    assert_eq!(resp.status().as_u16(), reqwest::StatusCode::OK);
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    app.get_confirmation_link(email_request)
}

async fn create_confirmed_subscriber(app: &TestAppWithCookie) {
    let confirmation_link = create_unconfirmed_subscriber(app).await.html;
    app.confirm_subscription(confirmation_link.as_str())
        .await
        .expect("fail to confirm subscription for a mock user");
}
