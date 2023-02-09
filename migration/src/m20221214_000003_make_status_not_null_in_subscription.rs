use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement, TransactionTrait},
};

use crate::m20220101_000001_create_subscription::Subscriptions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum UpdateSubscriptions {
    Status,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let txn = db.begin().await?;

        let sql = Query::update()
            .table(Subscriptions::Table)
            .and_where(Expr::col(UpdateSubscriptions::Status).is_null())
            .value(UpdateSubscriptions::Status, "confirmed".to_owned())
            .to_owned()
            .to_string(PostgresQueryBuilder);
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        db.execute(stmt).await.map(|_| ())?;

        manager
            .alter_table(
                Table::alter()
                    .table(Subscriptions::Table)
                    .modify_column(
                        ColumnDef::new(UpdateSubscriptions::Status)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        txn.commit().await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Subscriptions::Table)
                    .modify_column(ColumnDef::new(UpdateSubscriptions::Status).string().null())
                    .to_owned(),
            )
            .await
    }
}
