use anyhow::Context;
use poem::{handler, web::cookie::CookieJar, IntoResponse};
use poem_openapi::Object;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    context::StateContext,
    domain::idempotency::{
        get_saved_response, save_response, try_processing, IdempotencyKey, NextAction,
    },
    routes::error::{see_other_with_cookie, BasicError},
    session_state::FLASH_KEY,
};

type PublishResult<T> = std::result::Result<T, BasicError>;

#[handler]
pub async fn publish_newsletter(
    context: poem::web::Data<&StateContext>,
    form: poem::web::Form<NewsletterForm>,
    user_id: poem::web::Data<&Uuid>,
) -> PublishResult<poem::Response> {
    // list all confirmed subscribers
    // ideally, we should let some workers to handle all the confirmed subscribers
    // asynchrounously
    let db = &context.db;
    let NewsletterForm {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key
        .try_into()
        .map_err(BasicError::interval_error)?;

    let mut tx = match try_processing(db, &idempotency_key, user_id.0)
        .await
        .map_err(BasicError::interval_error)?
    {
        NextAction::ContinueProcessing(tx) => tx,
        NextAction::ReturnSavedResponse(saved_resp) => {
            return Ok(saved_resp);
        }
    };

    let issue_id = insert_newsletter_issue(&mut tx, &title, &text_content, &html_content)
        .await
        .map_err(BasicError::interval_error)?;

    // this will generate a task for each confirmed subscriber email
    enqueue_delivery_tasks(&mut tx, issue_id)
        .await
        .context("fail to enqueue email delivery task")
        .map_err(BasicError::interval_error)?;

    if let Some(saved_resp) = get_saved_response(db, &idempotency_key, user_id.0)
        .await
        .context("fail to get saved response")
        .map_err(BasicError::interval_error)?
    {
        return Ok(saved_resp);
    }

    let resp = see_other_with_cookie(
        "/admin/newsletters",
        "The newsletter issue has been published!",
    )
    .into_response();
    let resp = save_response(tx, &idempotency_key, user_id.0, resp)
        .await
        .map_err(BasicError::interval_error)?;
    Ok(resp)
}

#[handler]
pub async fn get_newsletter_submit_form(cookiejar: &CookieJar) -> poem::web::Html<String> {
    let mut error = String::new();
    if let Some(cookie) = cookiejar.get(FLASH_KEY) {
        error = format!("<p><i>{}</i></p>", cookie.value_str().to_owned());
    }
    let idempotency_key = Uuid::new_v4().to_string();
    poem::web::Html(format!(
        include_str!("newsletter.html"),
        error, idempotency_key
    ))
}

// {
//   "title": "abc",
//   "content": {
//     "html": "xxx",
//     "text": "yyy",
//   }
// }
#[derive(Deserialize, Debug, Object)]
pub struct NewsletterForm {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

async fn insert_newsletter_issue(
    tx: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (
            newsletter_issue_id,
            title,
            text_content,
            html_content,
            published_at
            )
        values ($1, $2, $3, $4, now())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    )
    .execute(tx)
    .await?;
    Ok(newsletter_issue_id)
}

async fn enqueue_delivery_tasks(
    tx: &mut Transaction<'_, Postgres>,
    issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (
            newsletter_issue_id,
            subscriber_email
            )
        SELECT $1, email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
        issue_id
    )
    .execute(tx)
    .await?;
    Ok(())
}
