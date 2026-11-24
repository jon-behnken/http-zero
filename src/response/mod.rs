use crate::response::http_status_code::HttpStatusCode;

pub mod http_status_code;
pub struct Response {
    status_code: HttpStatusCode,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: HttpStatusCode, body: Vec<u8>) -> Self {
        Self { status_code, body }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = format!(
            "HTTP/1.1 {} {}\r\n\r\n",
            self.status_code as u16,
            self.status_code.reason()
        )
        .as_bytes()
        .to_vec();
        bytes.extend(self.body.clone()); // FIXME optimization -- avoid cloning
        bytes
    }
}
