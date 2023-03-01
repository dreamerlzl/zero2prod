use poem::{Error, Response};
use reqwest::{header::LOCATION, StatusCode};
use tokio::task::JoinHandle;

use crate::session_state::FLASH_KEY;

pub fn see_other(uri: &str) -> Error {
    Error::from_response(
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, uri)
            .finish(),
    )
}

pub fn see_other_with_cookie(uri: &str, cookie: &str) -> Error {
    Error::from_response(
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, uri)
            .header(
                "Set-Cookie",
                format!("{}={}; Max-Age=1; Secure; HttpOnly", FLASH_KEY, cookie),
            )
            .finish(),
    )
}

pub fn e500(e: &str, context: &str) -> poem::Error {
    tracing::error!(error = e, context);
    poem::Error::from_response(
        poem::Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .finish(),
    )
}

pub fn e400(prompt: String) -> poem::Error {
    poem::Error::from_response(
        poem::Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .content_type("text/html")
            .body(prompt),
    )
}

pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();
    tokio::task::spawn_blocking(move || current_span.in_scope(f))
}
