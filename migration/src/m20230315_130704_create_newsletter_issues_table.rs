use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
pub enum NewsletterIssues {
    Table,
    NewsletterIssueID,
    Title,
    HtmlContent,
    TextContent,
    PublishedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .create_table(
                Table::create()
                    .table(NewsletterIssues::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NewsletterIssues::NewsletterIssueID)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(NewsletterIssues::Title).string().not_null())
                    .col(
                        ColumnDef::new(NewsletterIssues::TextContent)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NewsletterIssues::HtmlContent)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NewsletterIssues::PublishedAt)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Replace the sample below with your own migration scripts
        manager
            .drop_table(Table::drop().table(NewsletterIssues::Table).to_owned())
            .await
    }
}
