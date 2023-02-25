use poem::session::Session;
use poem_openapi::{
    payload::{Form, Html},
    Object, OpenApi, OpenApiService,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{context::StateContext, session_state::USER_ID_KEY, utils::see_other};

pub struct Api {
    context: StateContext,
}

#[OpenApi]
impl Api {
    #[oai(path = "/password", method = "get")]
    pub async fn change_password_form(
        &self,
        session: &Session,
    ) -> Result<Html<String>, poem::Error> {
        let msg_html = "";
        if let Some(user_id) = session.get::<Uuid>(USER_ID_KEY) {
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
        if let Some(user_id) = session.get::<Uuid>(USER_ID_KEY) {
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
