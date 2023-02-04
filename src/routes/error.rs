use std::fmt::Display;

use poem_openapi::{payload::PlainText, ApiResponse};
use tracing::error;

#[derive(ApiResponse, Debug)]
#[oai(display)]
pub enum ApiErrorResponse {
    #[oai(status = 400)]
    BadRequest(PlainText<String>),

    #[oai(status = 500)]
    InternalServerError,
}

impl Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErrorResponse::BadRequest(e) => write!(f, "bad request: {}", e.as_str()),
            ApiErrorResponse::InternalServerError => write!(f, "internal server error"),
        }
    }
}

impl From<sea_orm::DbErr> for ApiErrorResponse {
    fn from(value: sea_orm::DbErr) -> Self {
        error!(error = value.to_string(), "database error");
        ApiErrorResponse::InternalServerError
    }
}

impl From<reqwest::Error> for ApiErrorResponse {
    fn from(value: reqwest::Error) -> Self {
        error!(error = value.to_string(), "database error");
        ApiErrorResponse::InternalServerError
    }
}
