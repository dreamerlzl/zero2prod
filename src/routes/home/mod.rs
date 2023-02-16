use poem::{handler, IntoResponse, Response};

#[handler]
pub fn home() -> Response {
    include_str!("home.html")
        .with_content_type("text/html")
        .into_response()
}
