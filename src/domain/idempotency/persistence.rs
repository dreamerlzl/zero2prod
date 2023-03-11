use anyhow::Context;
use poem::Response;
use reqwest::StatusCode;
use sea_orm::DatabaseConnection;
use sqlx::postgres::PgHasArrayType;
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
            resp_status_code,
            resp_body,
            resp_headers as "resp_headers: Vec<HeaderPairRecord>"
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

pub async fn save_response(
    db: &DatabaseConnection,
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
    let pool = db.get_postgres_connection_pool();
    sqlx::query_unchecked!(
        r#"
        insert into idempotency (
            user_id,
            idempotency_key,
            resp_status_code,
            resp_headers,
            resp_body,
            created_at
        )
        values ($1, $2, $3, $4, $5, now())
        "#,
        user_id,
        idempotency_key.as_ref(),
        status,
        headers,
        payload.as_ref(),
    )
    .execute(pool)
    .await?;
    let resp = Response::from_parts(parts, poem::Body::from_bytes(payload));
    Ok(resp)
}
