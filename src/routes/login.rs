use poem::{
    http::{header::LOCATION, status::StatusCode},
    session::Session,
    Endpoint,
};
use poem_openapi::{
    payload::{Form, Html, Response},
    Object, OpenApi, OpenApiService,
};
use secrecy::Secret;
use serde::Deserialize;
use tracing::instrument;

use super::add_tracing;
use crate::{
    auth::{validate_credentials, AuthError, Credentials},
    context::StateContext,
    session_state::{FLASH_KEY, USER_ID_KEY},
    utils::see_other_with_cookie,
};

pub struct Api {
    context: StateContext,
}

pub fn get_api_service(
    context: StateContext,
    server_url: &str,
) -> (OpenApiService<Api, ()>, impl Endpoint) {
    let api_service = OpenApiService::new(Api::new(context), "login", "0.1").server(server_url);
    let ui = api_service.swagger_ui();
    (api_service, ui)
}

#[OpenApi]
impl Api {
    #[instrument(name = "get login page", skip(self, cookiejar))]
    #[oai(path = "/", method = "get", transform = "add_tracing")]
    async fn get_login(&self, cookiejar: &poem::web::cookie::CookieJar) -> Html<String> {
        // here the name of the cookie must be the same with that during Setting
        // see https://github.com/poem-web/poem/blob/74e6dd3d2badaca4fea44fb66568d7e37f13e3a5/poem-openapi/tests/operation_param.rs
        // espeically "cookie_rename"
        let mut error = String::new();
        if let Some(cookie) = cookiejar.get(FLASH_KEY) {
            error = format!("<p><i>{}</i></p>", cookie.value_str().to_owned());
        } else {
            tracing::error!(name = FLASH_KEY, "no cookie entry with name");
        }

        Html(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {error}
    <form action="/login" method="post">
        <label>Username
            <input
                type="text"
                placeholder="Enter Username"
                name="username"
            >
        </label>
        <label>Password
            <input
                type="password"
                placeholder="Enter Password"
                name="password"
            >
        </label>
        <button type="submit">Login</button>
    </form>
</body>
</html>"#,
        ))
    }

    // poem_openapi doesn't support Redirect directly
    // for poem, we could use this https://docs.rs/poem/latest/poem/web/struct.Redirect.html
    // #[instrument(name = "user attemps to new login", skip(self, form))]
    #[oai(path = "/", method = "post", transform = "add_tracing")]
    async fn post_login(
        &self,
        form: Form<LoginFrom>,
        session: &Session,
    ) -> Result<Response<()>, poem::Error> {
        let credentials = Credentials {
            username: form.0.username,
            password: Secret::new(form.0.password),
        };
        match validate_credentials(&self.context.db, credentials).await {
            Ok(user_id) => {
                tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
                // to avoid session fixation attacks
                session.renew();
                session.set(USER_ID_KEY, user_id);
                Ok(Response::new(())
                    .status(StatusCode::SEE_OTHER)
                    .header(LOCATION, "/admin/dashboard"))
            }
            Err(e) => {
                let e = match e {
                    AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                    AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
                };
                Err(see_other_with_cookie("/login", &e.to_string()))
            }
        }
    }
}

#[derive(Debug, Deserialize, Object)]
pub struct LoginFrom {
    username: String,
    // Secret<> doesn't impl poem_openapi::types::Type
    password: String,
}

#[derive(Debug, thiserror::Error)]
enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("internal error")]
    UnexpectedError(#[from] anyhow::Error),
}

impl Api {
    fn new(context: StateContext) -> Self {
        Api { context }
    }
}
