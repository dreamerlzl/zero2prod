use poem::web::Data;
use poem_openapi::{payload::Html, OpenApi};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

use super::super::add_session_uid_check;
use crate::{
    context::StateContext,
    entities::user::{self, Entity as Users},
    routes::error::BasicError,
};

pub struct Api {
    context: StateContext,
}

type DashboardResult<T> = std::result::Result<T, BasicError>;

#[OpenApi]
impl Api {
    #[oai(
        path = "/dashboard",
        method = "get",
        transform = "add_session_uid_check"
    )]
    pub async fn admin_dashboard(&self, user_id: Data<&Uuid>) -> DashboardResult<Html<String>> {
        let user_id = user_id.0;
        match get_username(*user_id, &self.context.db).await {
            Ok(Some(username)) => Ok(Html(format!(
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
        <li><a href="/admin/newsletters">Send a newsletter</li>
        <li>
          <form name="logoutForm" action="/logout" method="post">
            <input type="submit" value="Logout">
          </form>
        </li>
    </ol>
</body>
</html>"#
            ))),
            Ok(None) => {
                let msg = format!("username not found for user_id: {}", user_id);
                Err(BasicError::auth_error(msg))
            }
            Err(e) => {
                let msg = format!("fail to get username: {e}");
                Err(BasicError::interval_error(msg))
            }
        }
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
