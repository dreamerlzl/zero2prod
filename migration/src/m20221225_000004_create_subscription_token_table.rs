use sea_orm_migration::prelude::*;
use sea_query::table::ColumnDef;

use super::m20220101_000001_create_subscription::Subscriptions;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
pub enum SubscriptionTokens {
    Table,
    #[iden = "subscriber_id"]
    SubscriberID,

    #[iden = "subscription_token"]
    SubscriptionToken,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SubscriptionTokens::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SubscriptionTokens::SubscriberID)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(SubscriptionTokens::SubscriptionToken)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("subscriber_id")
                    .from(SubscriptionTokens::Table, SubscriptionTokens::SubscriberID)
                    .to(Subscriptions::Table, Subscriptions::Id)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SubscriptionTokens::Table).to_owned())
            .await
    }
}
