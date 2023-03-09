use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        let db = manager.get_connection();
        db.execute_unprepared(
            "CREATE TYPE header_pair AS (
                name TEXT,
                value BYTEA
                );",
        )
        .await?;
        db.execute_unprepared(
            r#"create table idempotency (
                user_id uuid NOT NULL REFERENCES "user"(id),
                idempotency_key TEXT NOT NULL,
                resp_status_code SMALLINT NOT NULL,
                resp_headers header_pair[] NOT NULL,
                resp_body BYTEA NOT NULL,
                created_at timestamptz NOT NULL,
                PRIMARY KEY(user_id, idempotency_key)
                );"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS idempotency;")
            .await?;
        db.execute_unprepared("DROP TYPE IF EXISTS header_pair;")
            .await?;
        Ok(())
    }
}
