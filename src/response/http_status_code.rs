#[derive(Clone, Debug, PartialEq)]
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

impl HttpStatusCode {
    pub fn reason(&self) -> &str {
        match self {
            HttpStatusCode::Ok => "Ok",
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
