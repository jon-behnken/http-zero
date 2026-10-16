#[derive(Clone, Debug, PartialEq)]
#[repr(u16)]
pub enum HttpStatusCode {
    Ok = 200,
    Created = 201,
    BadRequest = 400,
    NotFound = 404,
    InternalServerError = 500,
}

impl HttpStatusCode {
    pub fn reason(status_code: &HttpStatusCode) -> String {
        match status_code {
            HttpStatusCode::Ok => String::from("OK"),
            HttpStatusCode::Created => String::from("Created"),
            HttpStatusCode::NotFound => String::from("Not Found"),
            HttpStatusCode::BadRequest => String::from("Bad Request"),
            HttpStatusCode::InternalServerError => String::from("Internal Server Error"),
        }
    }
}
