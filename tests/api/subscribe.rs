use std::error::Error;

use anyhow::Result;
use poem::http::StatusCode;
use sea_orm::*;
use serial_test::serial;
use tracing::error;
use tracing_test::traced_test;
use zero2prod::entities::subscription;

use crate::api::get_client_and_db;

#[tokio::test]
#[traced_test]
#[serial]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<()> {
    let (cli, db) = get_client_and_db().await?;
    let valid_data = [
        "username=lzl&email=lzl2@gmail.com",
        "username=foo&email=bar@qq.com",
    ];

    for data in valid_data.into_iter() {
        let resp = cli
            .post("/subscription")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await;
        resp.assert_status(StatusCode::OK);
        let resp_json = resp.json().await;
        let id = resp_json.value().object().get("id").i64() as i32;
        let new_user = subscription::ActiveModel {
            id: ActiveValue::Set(id),
            ..Default::default()
        };
        if let Err(e) = new_user.delete(&db).await {
            error!(error = e.source(), id = id, "fail to delete test data");
        }
    }
    Ok(())
}

#[tokio::test]
#[traced_test]
#[serial]
async fn subscribe_returns_400_for_invalid_data() -> Result<()> {
    let (cli, _) = get_client_and_db().await?;
    let invalid_data = ["", "name=lzl", "email=aaa", "name=lzl&email=aaa", "foobar"];

    for data in invalid_data.into_iter() {
        let resp = cli
            .post("/subscription")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(data)
            .send()
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);
    }
    Ok(())
}
