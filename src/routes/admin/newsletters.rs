use anyhow::anyhow;
use poem::{handler, web::cookie::CookieJar, IntoResponse};
use poem_openapi::Object;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DeriveColumn, EntityTrait, EnumIter, QueryFilter, QuerySelect,
};
use serde::Deserialize;
use uuid::Uuid;

use super::super::subscriptions::ConfirmStatus;
use crate::{
    context::StateContext,
    domain::{
        idempotency::{get_saved_response, save_response, IdempotencyKey},
        Email,
    },
    entities::subscriptions::{self, Entity as Subscriptions},
    routes::ApiErrorResponse,
    session_state::FLASH_KEY,
    utils::see_other_with_cookie2,
};

#[handler]
pub async fn publish_newsletter(
    context: poem::web::Data<&StateContext>,
    form: poem::web::Form<NewsletterForm>,
    user_id: poem::web::Data<&Uuid>,
) -> Result<poem::Response, ApiErrorResponse> {
    // list all confirmed subscribers
    // ideally, we should let some workers to handle all the confirmed subscribers
    // asynchrounously
    let db = &context.db;
    let email_client = &context.email_client;
    let subscribers = get_confirmed_subscribers(db).await?;
    let NewsletterForm {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.0;
    let idempotency_key: IdempotencyKey = idempotency_key
        .try_into()
        .map_err(|_| PublishError::BadIdemPotencyKey)?;
    if let Some(saved_resp) = get_saved_response(db, &idempotency_key, user_id.0)
        .await
        .map_err(PublishError::GetSavedResponseFail)?
    {
        return Ok(saved_resp);
    }
    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await?;
            }
            Err(error) => {
                tracing::warn!(error.cause_chain=?error, "skipping a confirmed subscriber due to invalid email stored")
            }
        }
    }
    let resp = see_other_with_cookie2(
        "/admin/newsletters",
        "The newsletter issue has been published!",
    )
    .into_response();
    let resp = save_response(db, &idempotency_key, user_id.0, resp)
        .await
        .map_err(|e| {
            tracing::error!(
                error = e.to_string(),
                "internal error during saving response for retry-safety"
            );
            ApiErrorResponse::InternalServerError
        })?;
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

struct ConfirmedSubscriber {
    email: Email,
}

#[derive(thiserror::Error, Debug)]
pub enum PublishError {
    #[error("fail to get saved response")]
    GetSavedResponseFail(#[source] anyhow::Error),

    #[error("invalid idempotency key")]
    BadIdemPotencyKey,

    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),

    #[error("unexpected error")]
    UnexpectedError(#[from] anyhow::Error),
}

async fn get_confirmed_subscribers(
    db: &DatabaseConnection,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sea_orm::DbErr> {
    // select only one column without using a struct; ugly
    #[derive(Debug, Copy, Clone, EnumIter, DeriveColumn)]
    enum QueryAs {
        Email,
    }
    let subscribers = Subscriptions::find()
        .filter(subscriptions::Column::Status.eq(ConfirmStatus::Confirmed.to_string()))
        .select_only()
        .column(subscriptions::Column::Email)
        .into_values::<_, QueryAs>()
        .all(db)
        .await?
        // when we first store the subscribers' email, the app could be version X
        // when we later fetch and parse the email, the app could be version Y
        // email validation logic may change between these 2 versions
        .into_iter()
        .map(|r| match Email::parse(r) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(err) => Err(anyhow!(err)),
        })
        .collect();
    Ok(subscribers)
}
