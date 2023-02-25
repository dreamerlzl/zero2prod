use anyhow::Result;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use super::helpers::{assert_is_redirect_to, get_test_app_with_cookie};

#[sqlx::test]
async fn must_be_logged_in_to_see_the_change_password_form(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app_with_cookie(pool).await?;
    let resp = app.get_change_password().await;
    assert_is_redirect_to(&resp, "/login");
    Ok(())
}

#[sqlx::test]
async fn must_be_logged_in_to_change_your_password(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app_with_cookie(pool).await?;
    let new_password = Uuid::new_v4().to_string();
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;
    assert_is_redirect_to(&resp, "/login");
    Ok(())
}
