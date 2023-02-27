use reqwest::{header::LOCATION, StatusCode};

use crate::session_state::FLASH_KEY;

pub fn see_other(uri: &str) -> poem::Error {
    let resp = poem::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(LOCATION, uri)
        .finish();
    poem::Error::from_response(resp)
}

pub fn see_other_with_cookie(uri: &str, cookie: &str) -> poem::Error {
    let resp = poem::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(LOCATION, uri)
        .header(
            "Set-Cookie",
            format!("{}={}; Max-Age=1; Secure; HttpOnly", FLASH_KEY, cookie),
        )
        .finish();
    poem::Error::from_response(resp)
}

pub fn e500(e: &str, context: &str) -> poem::Error {
    tracing::error!(error = e, context);
    poem::Error::from_response(
        poem::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish(),
    )
}
