use anyhow::{anyhow, Context};
use poem::web::Data;
use poem_openapi::{
    payload::{Form, Html},
    Object, OpenApi,
};
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection};
use secrecy::Secret;
use serde::Deserialize;
use uuid::Uuid;

use super::dashboard::get_username;
use crate::{
    auth::{get_hash, validate_credentials, AuthError, Credentials},
    context::StateContext,
    entities::user,
    routes::{add_session_uid_check, error::BasicError},
    session_state::FLASH_KEY,
    utils::spawn_blocking_with_tracing,
};

type PasswordResult<T> = std::result::Result<T, BasicError>;

pub struct Api {
    context: StateContext,
}

#[OpenApi]
impl Api {
    #[oai(
        path = "/password",
        method = "get",
        transform = "add_session_uid_check"
    )]
    pub async fn change_password_form(
        &self,
        cookiejar: &poem::web::cookie::CookieJar,
    ) -> Html<String> {
        let mut msg_html = String::new();
        if let Some(cookie) = cookiejar.get(FLASH_KEY) {
            msg_html.push_str(&format!("<p><i>{}</i></p>", cookie.value_str()));
        }
        Html(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Change Password</title>
</head>
<body>
    {msg_html}
    <form action="/admin/password" method="post">
        <label>Current password
            <input
                type="password"
                placeholder="Enter current password"
                name="current_password"
            >
        </label>
        <br>
        <label>New password
            <input
                type="password"
                placeholder="Enter new password"
                name="new_password"
            >
        </label>
        <br>
        <label>Confirm new password
            <input
                type="password"
                placeholder="Type the new password again"
                name="new_password_check"
            >
        </label>
        <br>
        <button type="submit">Change password</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
        ))
    }

    #[oai(
        path = "/password",
        method = "post",
        transform = "add_session_uid_check"
    )]
    pub async fn change_password(
        &self,
        form: Form<ChangePasswordForm>,
        user_id: Data<&Uuid>,
    ) -> PasswordResult<()> {
        if form.new_password.len() > 128 || form.new_password.len() < 12 {
            return Err(BasicError::see_other(
                "/admin/password",
                "The password length must be between 12 to 128",
            ));
        }
        if form.new_password != form.new_password_check {
            return Err(BasicError::see_other(
                "/admin/password",
                "You entered two different new passwords - the field values must match",
            ));
        }
        let user_id = user_id.0;
        let username = get_username(*user_id, &self.context.db)
            .await
            .context("fail to get username from user_id")
            .map_err(BasicError::interval_error)?
            .ok_or(BasicError::bad_request(
                "no username found for id".to_owned(),
            ))?;
        let credentials = Credentials {
            username,
            password: Secret::new(form.current_password.clone()),
        };
        match validate_credentials(&self.context.db, credentials).await {
            Err(e) => match e {
                AuthError::InvalidCredentials(_) => Err(BasicError::see_other(
                    "/admin/password",
                    "The current password is incorrect",
                )),
                AuthError::UnexpectedError(_) => Err(BasicError::interval_error(e)),
            },
            Ok(uid) => {
                change_password(uid, form.new_password.clone(), &self.context.db)
                    .await
                    .map_err(BasicError::interval_error)?;
                Err(BasicError::see_other(
                    "/admin/password",
                    "Your password has been changed.",
                ))
            }
        }
    }
}

pub async fn change_password(
    uid: Uuid,
    password: String,
    db: &DatabaseConnection,
) -> Result<(), anyhow::Error> {
    // generate a random new salt
    let password_hashed =
        spawn_blocking_with_tracing(move || -> Result<String, argon2::password_hash::Error> {
            // let salt = SaltString::generate(&mut rand::thread_rng()).to_string();
            let salt = Uuid::new_v4().to_string();
            get_hash(&password, &salt)
        })
        .await
        .map_err(|e| {
            anyhow!(format!(
                "fail to join spawning tokio task for computing hash {e}"
            ))
        })??;

    let active_user = user::ActiveModel {
        id: ActiveValue::Set(uid),
        password_hashed: ActiveValue::Set(password_hashed),
        user_name: ActiveValue::NotSet,
    };
    active_user.update(db).await?;
    Ok(())
}

// Secret doesn't implement poem_openapi::types::Type
#[derive(Debug, Object, Deserialize)]
pub struct ChangePasswordForm {
    current_password: String,
    new_password: String,
    new_password_check: String,
}

impl Api {
    pub fn new(context: StateContext) -> Self {
        Self { context }
    }
}
