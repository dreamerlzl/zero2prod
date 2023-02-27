use poem::session::Session;
use poem_openapi::{payload::Html, OpenApi, OpenApiService};
use reqwest::{header::LOCATION, StatusCode};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{error, warn};
use uuid::Uuid;

use super::super::add_tracing;
use crate::{
    context::StateContext,
    entities::user::{self, Entity as Users},
    session_state::USER_ID_KEY,
    // routes::ApiErrorResponse,
};

pub struct Api {
    context: StateContext,
}

#[OpenApi]
impl Api {
    #[oai(path = "/dashboard", method = "get", transform = "add_tracing")]
    pub async fn admin_dashboard(&self, session: &Session) -> Result<Html<String>, poem::Error> {
        if let Some(user_id) = session.get::<Uuid>(USER_ID_KEY) {
            match get_username(user_id, &self.context.db).await {
                Ok(Some(username)) => {
                    return Ok(Html(format!(
                        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}!</p>
    <p>Available actions:</p>
    <ol>
        <li><a href="/admin/password">Change password</a></li>
        <li>
          <form name="logoutForm" action="/logout" method="post">
            <input type="submit" value="Logout">
          </form>
        </li>
    </ol>
</body>
</html>"#
                    )))
                }
                Ok(None) => {
                    warn!(
                        user_id = user_id.to_string(),
                        "username not found for user_id"
                    );
                    return Err(poem::Error::from_response(
                        poem::Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .finish(),
                    ));
                }
                Err(e) => {
                    error!(error = e.to_string(), "fail to get username");
                    return Err(poem::Error::from_response(
                        poem::Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .finish(),
                    ));
                }
            };
        }
        Err(poem::Error::from_response(
            poem::Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(LOCATION, "/login")
                .content_type("text/html")
                .finish(),
        ))
    }
}

impl Api {
    pub fn new(context: StateContext) -> Self {
        Self { context }
    }
}

pub async fn get_username(
    user_id: Uuid,
    db: &DatabaseConnection,
) -> Result<Option<String>, sea_orm::DbErr> {
    if let Some(user) = Users::find()
        .filter(user::Column::Id.eq(user_id))
        .one(db)
        .await?
    {
        return Ok(Some(user.user_name));
    }
    Ok(None)
}
