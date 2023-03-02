#[macro_export]
macro_rules! cookie_test {
    ($name:ident, [$x:ident] $fun:block) => {
        #[sqlx::test]
        async fn $name(pool: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            let $x = $crate::api::helpers::get_test_app_with_cookie(pool).await?;
            $fun
            Ok(())
        }
    };
}

#[macro_export]
macro_rules! login_test {
    ($name:ident, [$app:ident] $fun:block) => {
        $crate::cookie_test!($name, [$app] {
            let body = serde_json::json!({
                "username": $app.test_user.username,
                "password": $app.test_user.password,
            });
            $app.post_login(&body).await?;
            $fun
        });
    };
}

#[macro_export]
macro_rules! normal_test {
    ($name:ident, [$x:ident] $fun:block) => {
        #[sqlx::test]
        async fn $name(pool: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            let $x = $crate::api::helpers::get_test_app(pool).await?;
            $fun
            Ok(())
        }
    };
}
