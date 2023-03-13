use poem::{handler, session::Session, IntoResponse};
use reqwest::{header::LOCATION, StatusCode};

use crate::session_state::FLASH_KEY;

#[handler]
pub async fn post_logout(session: &Session) -> impl IntoResponse {
    tracing::info!("logout success in pure handler");
    session.purge();
    poem::Response::builder()
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
}
