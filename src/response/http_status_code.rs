use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u16)]
pub enum HttpStatusCode {
    Ok = 200,
    Created = 201,
    BadRequest = 400,
    NotFound = 404,
    LengthRequired = 411,
    ContentTooLarge = 413,
    InternalServerError = 500,
    NotImplemented = 501,
}

impl Display for HttpStatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u16)
    }
}

impl HttpStatusCode {
    pub fn reason(&self) -> &'static str {
        match self {
            HttpStatusCode::Ok => "OK",
            HttpStatusCode::Created => "Created",
            HttpStatusCode::NotFound => "Not Found",
            HttpStatusCode::BadRequest => "Bad Request",
            HttpStatusCode::InternalServerError => "Internal Server Error",
            HttpStatusCode::NotImplemented => "Not Implemented",
            HttpStatusCode::LengthRequired => "Length Required",
            HttpStatusCode::ContentTooLarge => "Content Too Large",
        }
    }
}
