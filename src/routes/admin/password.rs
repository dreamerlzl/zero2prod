use poem::session::Session;
use poem_openapi::{
    payload::{Form, Html},
    Object, OpenApi,
};
use secrecy::Secret;
use serde::Deserialize;
use uuid::Uuid;

use super::dashboard::get_username;
use crate::{
    auth::{validate_credentials, AuthError, Credentials},
    context::StateContext,
    session_state::{FLASH_KEY, USER_ID_KEY},
    utils::{e500, see_other, see_other_with_cookie},
};

pub struct Api {
    context: StateContext,
}

#[OpenApi]
impl Api {
    #[oai(path = "/password", method = "get")]
    pub async fn change_password_form(
        &self,
        cookiejar: &poem::web::cookie::CookieJar,
        session: &Session,
    ) -> Result<Html<String>, poem::Error> {
        let mut msg_html = String::new();
        if let Some(cookie) = cookiejar.get(FLASH_KEY) {
            msg_html.push_str(&format!("<p><i>{}</i></p>", cookie.value_str()));
        } else {
            tracing::info!("no flash message found");
        }
        if let Some(_user_id) = session.get::<Uuid>(USER_ID_KEY) {
        } else {
            return Err(see_other("/login"));
        }
        Ok(Html(format!(
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
        )))
    }

    #[oai(path = "/password", method = "post")]
    pub async fn change_password(
        &self,
        form: Form<ChangePasswordForm>,
        session: &Session,
    ) -> Result<(), poem::Error> {
        if form.new_password.len() > 128 || form.new_password.len() < 12 {
            return Err(see_other_with_cookie(
                "/admin/password",
                "The password length must be between 12 to 128",
            ));
        }
        if form.new_password != form.new_password_check {
            return Err(see_other_with_cookie(
                "/admin/password",
                "You entered two different new passwords - the field values must match",
            ));
        }
        if let Some(user_id) = session.get::<Uuid>(USER_ID_KEY) {
            let username = get_username(user_id, &self.context.db)
                .await
                .map_err(|e| e500(&e.to_string(), "fail to get username from id"))?
                .ok_or(e500("no username found for id", ""))?;
            let credentials = Credentials {
                username,
                password: Secret::new(form.current_password.clone()),
            };
            if let Err(e) = validate_credentials(&self.context.db, credentials).await {
                match e {
                    AuthError::InvalidCredentials(_) => {
                        return Err(see_other_with_cookie(
                            "/admin/password",
                            "The current password is incorrect",
                        ));
                    }
                    AuthError::UnexpectedError(_) => {
                        return Err(e500(&e.to_string(), ""));
                    }
                }
            }
        } else {
            return Err(see_other("/login"));
        }
        todo!()
    }
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
