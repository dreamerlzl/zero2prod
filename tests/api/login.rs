use std::collections::HashSet;

use anyhow::Result;
use poem::http::HeaderValue;
use reqwest::StatusCode;
use sqlx::{Pool, Postgres};

use super::helpers::get_test_app;

#[sqlx::test]
async fn an_error_flash_message_is_set_on_failure(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app(pool).await?;
    let body = serde_json::json!(
    {
        "username": "random-username",
        "password": "random-password",
    }
    );
    let resp = app.post_login(&body).await;
    resp.assert_status(StatusCode::SEE_OTHER);
    #[allow(clippy::mutable_key_type)]
    let cookies: HashSet<_> = resp.0.headers().get_all("Set-Cookie").into_iter().collect();
    assert!(cookies.contains(&HeaderValue::from_str("_flash=Authentication failed")?));
    // poem's TestClient doesn't support cookie store
    //let html_page = app.get_login_html().await;
    //println!("{:?}", &html_page);
    //assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
    Ok(())
}
