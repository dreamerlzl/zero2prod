use std::time::Duration;

use anyhow::anyhow;
use sea_orm::{DatabaseConnection, EntityTrait};
use sqlx::{Postgres, Transaction};
use tracing::{field::display, Span};
use uuid::Uuid;

use crate::{
    configuration::Configuration,
    domain::Email,
    email_client::EmailClient,
    entities::{newsletter_issues, prelude::NewsletterIssues},
    get_database_connection, get_email_client,
};

type IssueDeliveryResult<T> = std::result::Result<T, anyhow::Error>;

#[tracing::instrument(skip_all)]
pub async fn run_worker_until_stop(config: Configuration) -> IssueDeliveryResult<()> {
    let db = get_database_connection(config.db).await?;
    let email_client = get_email_client(config.email_client)?;
    worker_loop(&db, &email_client).await
}

#[tracing::instrument(skip_all)]
async fn worker_loop(
    db: &DatabaseConnection,
    email_client: &EmailClient,
) -> IssueDeliveryResult<()> {
    loop {
        match try_execute_task(db, email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

// NOTE: maybe we should use a pub/sub mw like kafka and divide all confirmed subscriber emails across different messages in the **same** batch
#[tracing::instrument(
    skip_all,
    fields(
        newsletter_issue_id=tracing::field::Empty,
        subscriber_email=tracing::field::Empty
    ),
    err
)]
pub async fn try_execute_task(
    db: &DatabaseConnection,
    email_client: &EmailClient,
) -> IssueDeliveryResult<ExecutionOutcome> {
    let Some((tx, issue_id, subscriber_email)) = dequeue_task(db).await? else {
        return Ok(ExecutionOutcome::EmptyQueue);
    };
    Span::current()
        .record("newsletter_issue_id", &display(issue_id))
        .record("subscriber_email", &display(&subscriber_email));
    // send the email
    // we need to get the email content from the issue_id
    let Some(issue) = get_issue(db, issue_id).await? else {
        return Err(anyhow!("no issue found for issue id: {}", issue_id));
    };

    let email = match Email::parse(subscriber_email.clone()) {
        Ok(email) => email,
        Err(e) => {
            tracing::error!(error.cause_chain=?e, error.message=%e, "skipping a confirmed subscriber due to the invalid email");
            return Err(anyhow::anyhow!(e));
        }
    };

    // TODO: add retry
    if let Err(e) = email_client
        .send_email(
            &email,
            &issue.title,
            &issue.html_content,
            &issue.text_content,
        )
        .await
    {
        tracing::error!(error.cause_chain=?e, error.message=%e, "failed to deliver issue to a confirmed subscriber");
    }

    delete_task(tx, issue_id, &subscriber_email).await?;
    Ok(ExecutionOutcome::TaskCompleted)
}

type PgTransaction = Transaction<'static, Postgres>;
// type PgTransaction = sea_orm::DatabaseTransaction;

#[tracing::instrument(skip_all)]
async fn dequeue_task(
    db: &DatabaseConnection,
) -> IssueDeliveryResult<Option<(PgTransaction, Uuid, String)>> {
    // let mut tx = db.begin().await.context("fail to get a transaction")?;
    let pool = db.get_postgres_connection_pool();
    let mut tx = pool.begin().await?;
    let r = sqlx::query!(
        r#"
        select newsletter_issue_id, subscriber_email
        from issue_delivery_queue
        for update
        skip locked
        limit 1
        "#
    )
    .fetch_optional(&mut tx)
    .await?;
    if let Some(r) = r {
        Ok(Some((tx, r.newsletter_issue_id, r.subscriber_email)))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut tx: PgTransaction,
    issue_id: Uuid,
    subscriber_email: &str,
) -> IssueDeliveryResult<()> {
    sqlx::query!(
        r#"
        delete from issue_delivery_queue
        where
            newsletter_issue_id = $1 and
            subscriber_email = $2
        "#,
        issue_id,
        subscriber_email
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn get_issue(
    db: &DatabaseConnection,
    issue_id: Uuid,
) -> IssueDeliveryResult<Option<newsletter_issues::Model>> {
    let issue = NewsletterIssues::find_by_id(issue_id).one(db).await?;
    Ok(issue)
}
