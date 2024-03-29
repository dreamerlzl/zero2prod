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

use super::{add_tracing, error::BasicError};
use crate::{
    auth::{validate_credentials, Credentials},
    context::StateContext,
    session_state::{FLASH_KEY, USER_ID_KEY},
};

type LoginResult<T> = std::result::Result<T, BasicError>;

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
    ) -> LoginResult<Response<()>> {
        let credentials = Credentials {
            username: form.0.username.clone(),
            password: Secret::new(form.0.password),
        };
        match validate_credentials(&self.context.db, credentials).await {
            Ok(user_id) => {
                tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
                // to avoid session fixation attacks
                session.renew();
                session.set(USER_ID_KEY, user_id);
                tracing::info!(
                    username = form.0.username,
                    user_id = user_id.to_string(),
                    "login user uid is"
                );
                Ok(Response::new(())
                    .status(StatusCode::SEE_OTHER)
                    .header(LOCATION, "/admin/dashboard"))
            }
            Err(e) => Err(BasicError::see_other("/login", &e.to_string())),
        }
    }
}

#[derive(Debug, Deserialize, Object)]
pub struct LoginFrom {
    username: String,
    // Secret<> doesn't impl poem_openapi::types::Type
    password: String,
}

impl Api {
    fn new(context: StateContext) -> Self {
        Api { context }
    }
}
