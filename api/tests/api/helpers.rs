use std::sync::Arc;

use anyhow::{Context, Result};
use fake::{faker::internet::en::SafeEmail, Fake};
use linkify::LinkFinder;
use migration::{Migrator, MigratorTrait};
use once_cell::sync::Lazy;
use poem::test::{TestClient, TestResponse};
use poem::{Body, Route};
use rand::distributions::{Alphanumeric, DistString};
use sea_orm::*;
use sea_orm::{query::Statement, DatabaseConnection, DeleteResult};
use sea_orm_migration::prelude::*;
use sqlx::PgPool;
use tracing::{info, warn};
use wiremock::MockServer;
use zero2prod_api::configuration::{get_test_configuration, RelationalDBSettings};
use zero2prod_api::context::Context as RouteContext;
use zero2prod_api::domain::Email;
use zero2prod_api::entities::subscriptions;
use zero2prod_api::routes::default_route;
use zero2prod_api::{get_database_connection, setup_logger};

pub async fn post_subscription<T: 'static + Into<Body>>(
    cli: &TestClient<Route>,
    data: T,
) -> TestResponse {
    cli.post("/subscriptions")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(data)
        .send()
        .await
}

pub fn email() -> Email {
    Email::parse(SafeEmail().fake::<String>()).unwrap()
}

pub fn get_first_link(text: &str) -> String {
    let finder = LinkFinder::new();
    let links: Vec<_> = finder
        .links(text)
        .filter(|l| *l.kind() == linkify::LinkKind::Url)
        .collect();
    assert_eq!(links.len(), 1);
    links[0].as_str().to_owned()
}

pub async fn delete_subscriber_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<DeleteResult, DbErr> {
    subscriptions::Entity::delete_by_id(id).exec(db).await
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info";
    if std::env::var("TEST_LOG").is_ok() {
        setup_logger(default_filter_level);
    }
});

pub struct TestApp {
    pub cli: TestClient<Route>,
    pub db: DatabaseConnection,
    pub email_server: MockServer,
    pub database_name: String,
}

//impl Drop for TestApp {
//    fn drop(&mut self) {
//        info!(database_name = self.database_name, "cleaning up database");
//        let database_name = self.database_name.clone();
//        let backend = self.db.get_database_backend();
//        let db = self.db.clone();
//
//        std::thread::scope(|s| {
//            s.spawn(|| {
//                let runtime = tokio::runtime::Builder::new_multi_thread()
//                    .enable_all()
//                    .build()
//                    .unwrap();
//                runtime.block_on(async move {
//                    let stmt = Statement::from_string(
//                        backend,
//                        format!(r#"drop database "{}" ;"#, database_name),
//                    );
//                    if let Err(e) = db.execute(stmt).await {
//                        warn!(
//                            error = e.to_string(),
//                            database_name = database_name,
//                            "fail to cleanup database"
//                        );
//                    }
//                });
//            });
//        });
//    }
//}

pub async fn get_test_app() -> Result<TestApp> {
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;
    let mut conf = get_test_configuration("config/test").expect("fail to get conf");
    conf.email_client.api_base_url = email_server.uri();
    let database_name = random_db_name();
    conf.db.name = database_name.clone();
    create_and_migrate_db(conf.db.clone()).await?;
    let context = RouteContext::new(conf.clone()).await?;
    let db = context.db.clone();

    let context = Arc::new(context);
    let app = default_route(conf, context).await;
    let cli = TestClient::new(app);
    Ok(TestApp {
        cli,
        db,
        email_server,
        database_name,
    })
}

fn random_db_name() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
}

async fn create_and_migrate_db(sql: RelationalDBSettings) -> Result<()> {
    let options = sql.options_without_db();
    let pool = PgPool::connect_with(options)
        .await
        .context("fail to connect to pg")?;
    let conn_without_db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
    let stmt = Statement::from_string(
        conn_without_db.get_database_backend(),
        format!(r#"CREATE DATABASE "{}"; "#, sql.name),
    );
    conn_without_db.execute(stmt).await?;

    let conn_with_db = get_database_connection(sql).await?;
    // migrate db
    Migrator::refresh(&conn_with_db).await?;
    Ok(())
}
