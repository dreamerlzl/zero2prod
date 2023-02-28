use poem::{handler, session::Session, IntoResponse};
use reqwest::{header::LOCATION, StatusCode};
use uuid::Uuid;

use crate::{
    session_state::{FLASH_KEY, USER_ID_KEY},
    utils::see_other,
};

#[handler]
pub async fn post_logout(session: &Session) -> Result<impl IntoResponse, poem::Error> {
    if session.get::<Uuid>(USER_ID_KEY).is_some() {
        tracing::info!("logout success in pure handler");
        session.purge();
        let resp = poem::Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, "/login")
            .header(
                "Set-Cookie",
                format!(
                    "{}={}; Max-Age=1; Secure; HttpOnly",
                    FLASH_KEY, "You have successfully logged out."
                ),
            )
            .finish();
        Ok(resp)
    } else {
        Err(poem::Error::from_response(see_other("/login")))
    }
}
