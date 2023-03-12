use anyhow::Context;
use poem::Response;
use reqwest::StatusCode;
use sea_orm::DatabaseConnection;
use sqlx::{postgres::PgHasArrayType, Postgres, Transaction};
use uuid::Uuid;

use super::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_header_pair")
    }
}

pub async fn get_saved_response(
    db: &DatabaseConnection,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
) -> Result<Option<Response>, anyhow::Error> {
    let pool = db.get_postgres_connection_pool();
    let saved_response = sqlx::query!(
        r#"
        select
            resp_status_code as "resp_status_code!",
            resp_body as "resp_body!",
            resp_headers as "resp_headers!: Vec<HeaderPairRecord>"
        from idempotency
        where
            user_id = $1 and
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;
    if let Some(r) = saved_response {
        let status = StatusCode::from_u16(r.resp_status_code.try_into()?)?;
        let payload =
            String::from_utf8(r.resp_body).context("fail to convert Vec<u8> to String")?;
        let mut resp = Response::builder().status(status);
        for HeaderPairRecord { name, value } in r.resp_headers {
            resp = resp.header(name, value);
        }
        Ok(Some(resp.body(payload)))
    } else {
        Ok(None)
    }
}

#[tracing::instrument(skip(tx, user_id))]
pub async fn save_response(
    mut tx: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
    resp: Response,
) -> Result<Response, anyhow::Error> {
    let status = resp.status().as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(resp.headers().len());
        for (name, value) in resp.headers().iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };
    let (parts, body) = resp.into_parts();
    // we assume the memory can hold the whole body
    let payload = body.into_bytes().await?;
    sqlx::query_unchecked!(
        r#"
        UPDATE idempotency
        SET
            resp_status_code  = $3,
            resp_headers      = $4,
            resp_body         = $5
        WHERE
            user_id          = $1 AND
            idempotency_key  = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status,
        headers,
        payload.as_ref(),
    )
    .execute(&mut tx)
    .await?;
    tx.commit().await?;
    let resp = Response::from_parts(parts, poem::Body::from_bytes(payload));
    Ok(resp)
}

pub enum NextAction {
    ContinueProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(poem::Response),
}

#[tracing::instrument(skip(db, user_id))]
pub async fn try_processing(
    db: &DatabaseConnection,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
) -> Result<NextAction, anyhow::Error> {
    let pool = db.get_postgres_connection_pool();
    let mut transaction = pool.begin().await?;
    let num_inserted_rows = sqlx::query!(
        r#"
        insert into idempotency (
            user_id,
            idempotency_key,
            created_at
            )
        values ($1, $2, now())
        on conflict do nothing
        "#,
        user_id,
        idempotency_key.as_ref()
    )
    .execute(&mut transaction)
    .await?
    .rows_affected();
    if num_inserted_rows > 0 {
        Ok(NextAction::ContinueProcessing(transaction))
    } else {
        let saved_resp = get_saved_response(db, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("a saved response expected"))?;
        Ok(NextAction::ReturnSavedResponse(saved_resp))
    }
}
