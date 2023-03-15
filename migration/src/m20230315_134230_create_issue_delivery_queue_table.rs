use sea_orm_migration::prelude::*;

use crate::m20230315_130704_create_newsletter_issues_table::NewsletterIssues;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
pub enum IssueDeliveryQueue {
    Table,
    SubscriberEmail,
    NewsletterIssueID,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(IssueDeliveryQueue::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IssueDeliveryQueue::SubscriberEmail)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IssueDeliveryQueue::NewsletterIssueID)
                            .uuid()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("newsletter_issue_id")
                    .from(
                        IssueDeliveryQueue::Table,
                        IssueDeliveryQueue::NewsletterIssueID,
                    )
                    .to(NewsletterIssues::Table, NewsletterIssues::NewsletterIssueID)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(IssueDeliveryQueue::Table).to_owned())
            .await
    }
}
