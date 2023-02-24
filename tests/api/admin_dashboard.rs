use super::helpers::{assert_is_redirect_to, get_test_app_with_cookie};
use anyhow::Result;
use sqlx::{Pool, Postgres};

#[sqlx::test]
async fn must_be_logged_in_to_access_the_board(pool: Pool<Postgres>) -> Result<()> {
    let app = get_test_app_with_cookie(pool).await?;
    let resp = app.get_admin_dashboard().await?;
    assert_is_redirect_to(&resp, "/login");
    Ok(())
}
