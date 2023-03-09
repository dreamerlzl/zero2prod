use anyhow::Context;
use poem_openapi::{payload::Response, ApiResponse};
use reqwest::StatusCode;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use super::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub async fn get_saved_response(
    db: &DatabaseConnection,
    idempotency_key: &IdempotencyKey,
    user_id: &Uuid,
) -> Result<Option<Response<()>>, anyhow::Error> {
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
        let mut resp = Response::new(()).status(status);
        for HeaderPairRecord { name, value } in r.resp_headers {
            resp = resp.header(name, value);
        }
        Ok(Some(resp))
    } else {
        Ok(None)
    }
}
