use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::request::request_error::RequestError;
use crate::response::http_status_code::HttpStatusCode;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HttpMethod {
    Get,
    Options,
    Post,
    Put,
    Patch,
}

impl HttpMethod {
    pub fn length_required(&self) -> bool {
        let requires = [HttpMethod::Put, HttpMethod::Post, HttpMethod::Patch];
        requires.contains(self)
    }
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
        }
    }
}

impl FromStr for HttpMethod {
    type Err = RequestError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "GET" => Ok(HttpMethod::Get),
            "OPTIONS" => Ok(HttpMethod::Options),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "PATCH" => Ok(HttpMethod::Patch),
            _ => Err(RequestError {
                status_code: HttpStatusCode::NotImplemented,
            }),
        }
    }
}
