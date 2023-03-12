use poem::{Error, Response};
use poem_openapi::Object;
use reqwest::{header::LOCATION, StatusCode};
use serde::Serialize;

use crate::session_state::FLASH_KEY;

pub fn see_other_error(uri: &str) -> Error {
    Error::from_response(
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, uri)
            .finish(),
    )
}

pub fn see_other_with_cookie(uri: &str, cookie: &str) -> poem_openapi::payload::Response<()> {
    poem_openapi::payload::Response::new(())
        .status(StatusCode::SEE_OTHER)
        .header(LOCATION, uri)
        .header(
            "Set-Cookie",
            format!("{}={}; Max-Age=1; Secure; HttpOnly", FLASH_KEY, cookie),
        )
}

pub fn flash_message(value: &str) -> String {
    format!("{}={}; Max-Age=1; Secure; HttpOnly", FLASH_KEY, value)
}

#[derive(Debug, Serialize, Object)]
pub struct MyError {
    message: String,
}

impl MyError {
    pub fn new_error<E: std::fmt::Display>(message: E) -> Self {
        MyError {
            message: format!("{}", message),
        }
    }
}

impl ToString for MyError {
    fn to_string(&self) -> String {
        self.message.clone()
    }
}

#[macro_export]
macro_rules! generate_error_response {
    ($enum_name:ident, 303) => {
        #[derive(Debug, poem_openapi::ApiResponse)]
        pub enum $enum_name {
            #[oai(status = 303)]
            SeeOther(
                #[oai(header = "LOCATION")] String,
                #[oai(header = "Set-Cookie")] String,
            ),
        }

        impl $enum_name {
            pub fn see_other(location: &str, message: &str) -> Self {
                $enum_name::SeeOther(location.to_owned(), message.to_owned())
            }
        }
    };
    ($enum_name:ident, $(($status:literal, $name:ident)),*) => {
        #[derive(Debug, poem_openapi::ApiResponse)]
        pub enum $enum_name {
            #[oai(status = 303)]
            SeeOther(
                #[oai(header = "LOCATION")] String,
                #[oai(header = "Set-Cookie")] String,
            ),

            #[oai(status = 500)]
            InternalServerError(poem_openapi::payload::Json<$crate::routes::error::MyError>),

            $(
            #[oai(status = $status)]
            $name(poem_openapi::payload::Json<$crate::routes::error::MyError>),
            )*
        }

        impl $enum_name {
            paste::paste!{
                $(
                pub fn [<$name:snake>]<Err: std::fmt::Display>(
                    err: Err,
                ) -> Self {
                    $enum_name::$name(poem_openapi::payload::Json($crate::routes::error::MyError::new_error(err)))
                }
                )*
            }

            pub fn interval_error<Err: std::fmt::Display>(err: Err) -> Self {
                let error_msg = format!("{}", err);
                tracing::error!(error = error_msg, "internal server error");
                $enum_name::InternalServerError(poem_openapi::payload::Json(
                    $crate::routes::error::MyError::new_error(err),
                ))
            }

            pub fn see_other(location: &str, message: &str) -> Self {
                $enum_name::SeeOther(location.to_owned(), flash_message(message))
            }
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $enum_name::SeeOther(location, cookie_value) => {
                        write!(f, "redirect to {} with {}", location, cookie_value)
                    }
                    $enum_name::InternalServerError(_) => {
                        write!(f, "internal server error")
                    }
                    _ => write!(f, "{:?}", self),
                }
            }
        }
    };
}

generate_error_response!(BasicError, (400, BadRequest), (401, AuthError));
