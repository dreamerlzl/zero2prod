use std::fmt::Display;

use poem_openapi::{payload::PlainText, ApiResponse};
use thiserror::Error;
use tracing::error;

use super::newsletters::PublishError;

#[derive(ApiResponse, Debug, Error)]
#[oai(display)]
pub enum ApiErrorResponse {
    #[oai(status = 400)]
    BadRequest(PlainText<String>),

    #[oai(status = 401)]
    AuthError(#[oai(header = "WWW-Authenticate")] String),

    #[oai(status = 500)]
    InternalServerError,
}

impl Display for ApiErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiErrorResponse::BadRequest(e) => write!(f, "bad request: {}", e.as_str()),
            ApiErrorResponse::InternalServerError => write!(f, "internal server error"),
            ApiErrorResponse::AuthError(_) => write!(f, "authorization error"),
        }
    }
}

impl From<PublishError> for ApiErrorResponse {
    fn from(value: PublishError) -> Self {
        match value {
            PublishError::AuthError(_) => {
                ApiErrorResponse::AuthError(r#"Basic realm="publish""#.to_owned())
            }
            PublishError::UnexpectedError(e) => {
                error!(error = e.to_string(), "unexpected publish error");
                ApiErrorResponse::InternalServerError
            }
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
