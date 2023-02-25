use reqwest::{header::LOCATION, StatusCode};

pub fn see_other(uri: &str) -> poem::Error {
    let resp = poem::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(LOCATION, uri)
        .finish();
    poem::Error::from_response(resp)
}
