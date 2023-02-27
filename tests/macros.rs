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
