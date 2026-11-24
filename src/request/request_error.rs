use std::str::Utf8Error;
use std::string::FromUtf8Error;

use crate::response::http_status_code::HttpStatusCode;

#[derive(Debug, PartialEq)]
pub struct RequestError {
    pub status_code: HttpStatusCode,
}

impl RequestError {
    pub fn message(&self) -> &'static str {
        self.status_code.reason()
    }
}

impl From<FromUtf8Error> for RequestError {
    fn from(value: FromUtf8Error) -> Self {
        eprintln!("{}", value);
        RequestError {
            status_code: HttpStatusCode::BadRequest,
        }
    }
}

impl From<Utf8Error> for RequestError {
    fn from(value: Utf8Error) -> Self {
        eprintln!("{}", value);
        RequestError {
            status_code: HttpStatusCode::BadRequest,
        }
    }
}
