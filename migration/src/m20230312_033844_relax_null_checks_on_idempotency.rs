use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            ALTER TABLE idempotency ALTER COLUMN resp_status_code DROP NOT NULL;
            ALTER TABLE idempotency ALTER COLUMN resp_body DROP NOT NULL;
            ALTER TABLE idempotency ALTER COLUMN resp_headers DROP NOT NULL;
            "#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
            ALTER TABLE idempotency ALTER COLUMN resp_status_code SET NOT NULL;
            ALTER TABLE idempotency ALTER COLUMN resp_body        SET NOT NULL;
            ALTER TABLE idempotency ALTER COLUMN resp_headers     SET NOT NULL;
            "#,
        )
        .await?;
        Ok(())
    }
}
