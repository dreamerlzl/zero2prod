use anyhow::Context;

use super::helpers::assert_is_redirect_to;
use crate::cookie_test;

cookie_test!(an_error_flash_message_is_set_on_failure, [app] {
    let body = serde_json::json!(
    {
        "username": "random-username",
        "password": "random-password",
    }
    );
    let resp = app.post_login(&body).await?;
    assert_is_redirect_to(&resp, "/login");
    let flash_cookie = resp.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication failed");
    let html_page = app.get_login_html().await;
    assert!(
        html_page.contains(r#"<p><i>Authentication failed</i></p>"#),
        "{}",
        html_page
    );
    // Act - Part 3 - Reload the login page
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let html_page = app.get_login_html().await;
    assert!(
        !html_page.contains(r#"<p><i>Authentication failed</i></p>"#),
        "{}",
        html_page
    );
});

cookie_test!(redirect_to_admin_dashboard_after_login, [app] {
    let body = serde_json::json!({
        "username": app.test_user.username,
        "password": app.test_user.password,
    });
    let resp = app.post_login(&body).await.context("fail to post login")?;
    assert_is_redirect_to(&resp, "/admin/dashboard");
    let html_page = app.get_admin_dashboard().await.text().await?;
    assert!(
        html_page.contains(&format!("Welcome {}", app.test_user.username)),
        "the html page content is '{}'",
        html_page
    );
});
