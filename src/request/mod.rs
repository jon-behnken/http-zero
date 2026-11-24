use std::collections::HashMap;
use std::io::Read;
use std::str::{FromStr, from_utf8};

use crate::response::http_status_code::HttpStatusCode;

pub mod http_method;
mod request_error;

use http_method::HttpMethod;
use request_error::RequestError;

#[derive(Clone, Debug, PartialEq)]
pub struct Headers {
    pub accept: Option<String>,
    pub authorization: Option<String>,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub host: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Request {
    request_target: String,
    headers: Headers,
    method: HttpMethod,
    body: Vec<u8>,
}

const HTTP_HEADER_TERMINATOR: &[u8; 4] = b"\r\n\r\n";
const MAX_BODY_SIZE: u64 = 10 * 1024 * 1024; // 10 MB

/*
    Compile-time check to ensure MAX_BODY_SIZE can never
    be set to exceed the underlying architecture's storage capacity
*/
const _: () = assert!(MAX_BODY_SIZE < usize::MAX as u64);

#[allow(unused)]
impl Request {
    /*
       from_stream is the public constructor; it takes a TcpStream
       and calls consume() which reads bytes off the stream and
       parses them into headers, method and body
    */
    pub fn from_stream<T: Read>(stream: &mut T) -> Result<Self, RequestError> {
        Request::consume(stream)
    }

    pub fn request_target(&self) -> &str {
        &self.request_target
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn method(&self) -> &HttpMethod {
        &self.method
    }

    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }

    /*
       new() is a private constructor that reads the full HTTP headers
       into a buffer, as well as any residual bytes that belong to the body.
    */
    fn new<T: Read>(stream: &mut T) -> Result<Request, RequestError> {
        // Stores all bytes that belong to the header
        let mut headers_buffer: Vec<u8> = vec![];

        // Stores any bytes that are read beyond the HTTP_HEADER_TERMINATOR
        let mut body_buffer: Vec<u8> = vec![];

        let mut headers_read = false;

        while !headers_read {
            // This buffer is scoped to each iteration of the loop;
            // it reads a fixed-size number of bytes off the stream.
            //
            // We don't know ahead of time how large the header will be,
            // so we loop over the stream bytes and fill this buffer each time.
            // Once the scoped buffer is filled, we push those bytes onto the
            // top-level headers_buffer.
            let mut buf: [u8; 512] = [0; 512];
            match stream.read(&mut buf) {
                Err(e) => println!("Error reading into buffer: {:?}", e), // FIXME error_handling
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        return Err(RequestError {
                            status_code: HttpStatusCode::BadRequest,
                        });
                    }
                    // The stream may end up containing less bytes than we've allocated for the buffer.
                    // To ensure we only write valid data, slice the buffer from beginning to the last valid byte.
                    let bytes = &buf[0..bytes_read];
                    headers_buffer.extend(bytes);

                    // The HTTP_HEADER_TERMINATOR may arrive in sequential reads (e.g. one call may have '\r' and the next the remaining '\n\r'\n )
                    // So we need to scan the full headers_buffer, not just the scoped one.
                    let mut header_end_index: usize = 0;
                    for (i, window) in headers_buffer.windows(4).enumerate() {
                        if window == HTTP_HEADER_TERMINATOR {
                            headers_read = true;
                            header_end_index = i + 4;

                            // Again, we're reading a fixed number of bytes off the stream each time.
                            // It's possible to read past the header, and into the body. Any bytes that
                            // belong to the body must be stored in a buffer as well.
                            body_buffer = headers_buffer[header_end_index..].to_vec();
                        }
                    }
                    if headers_read {
                        headers_buffer = headers_buffer[0..header_end_index].to_vec()
                    }
                }
            }
        }

        let mut headers: HashMap<String, String> = HashMap::new();
        let mut header_key: Vec<u8> = vec![];
        let mut header_value: Vec<u8> = vec![];

        let mut reading_http_request_line = true;
        let mut request_line_delimiter_count: usize = 0;

        // Advance to the next key-value pair in headers
        let mut write_to_header_value = false;

        let mut method: Vec<u8> = vec![];
        let mut request_target: Vec<u8> = vec![];
        let mut version: Vec<u8> = vec![];

        let mut i: usize = 0;
        while i < headers_buffer.len() - 2 {
            let advance_pointer = &headers_buffer[i..i + 2] == b"\r\n";

            // A proper request line has two delimiter space characters
            if reading_http_request_line && request_line_delimiter_count > 2 {
                return Err(RequestError {
                    status_code: HttpStatusCode::BadRequest,
                });
            }

            // Finished reading request line
            if advance_pointer && reading_http_request_line {
                reading_http_request_line = false;
                i += 2;
                continue;
            }

            if reading_http_request_line {
                let record_http_method = request_line_delimiter_count == 0;
                let record_request_target = request_line_delimiter_count == 1;
                let record_http_version = request_line_delimiter_count == 2;

                if headers_buffer[i] == b' ' {
                    request_line_delimiter_count += 1;
                    i += 1;
                    continue;
                }
                if record_http_method {
                    method.push(headers_buffer[i]);
                    i += 1;
                    continue;
                };

                if record_request_target {
                    request_target.push(headers_buffer[i]);
                    i += 1;
                    continue;
                }

                if record_http_version {
                    version.push(headers_buffer[i]);
                    i += 1;
                    continue;
                }
            }

            // Transition to writing the header's value
            // Skip 2 bytes to exclude ': '
            if &headers_buffer[i..i + 2] == b": " {
                write_to_header_value = true;
                i += 2;
                continue;
            }

            // Insert current header key and value
            // Clear the buffer variables
            // Skip 2 bytes to exclude '\r\n'
            if advance_pointer {
                headers.insert(
                    String::from_utf8(header_key.clone().to_ascii_lowercase())?,
                    String::from_utf8(header_value.clone())?,
                );

                header_key.clear();
                header_value.clear();
                write_to_header_value = false;
                i += 2;
                continue;
            }

            if write_to_header_value {
                header_value.push(headers_buffer[i]);
            } else {
                header_key.push(headers_buffer[i]);
            }

            i += 1;
        }

        let method = HttpMethod::from_str(from_utf8(&method)?)?;
        let content_length: Result<Option<u64>, RequestError> =
            match headers.remove("content-length") {
                Some(c) => match c.parse::<u64>() {
                    Ok(c) => Ok(Some(c)),
                    Err(e) => Err(RequestError {
                        status_code: HttpStatusCode::BadRequest,
                    }),
                },
                None => {
                    if method.length_required() {
                        return Err(RequestError {
                            status_code: HttpStatusCode::LengthRequired,
                        });
                    };
                    Ok(None)
                }
            };

        let headers = Headers {
            authorization: headers.remove("authorization"),
            content_length: content_length?,
            content_type: headers.remove("content-type"),
            accept: headers.remove("accept"),
            host: headers.remove("host"),
            user_agent: headers.remove("user-agent"),
        };

        Ok(Request {
            request_target: String::from_utf8(request_target)?,
            headers,
            method,
            body: body_buffer,
        })
    }

    fn consume<T: Read>(stream: &mut T) -> Result<Self, RequestError> {
        match Request::new(stream) {
            Err(e) => Err(e),
            Ok(mut request) => {
                let declared_total_bytes: usize;
                if let Some(content_length) = request.headers.content_length {
                    if (content_length >= MAX_BODY_SIZE) {
                        let status_code = HttpStatusCode::ContentTooLarge;
                        return Err(RequestError { status_code });
                    }
                    declared_total_bytes =
                        usize::try_from(content_length).map_err(|_| RequestError {
                            status_code: HttpStatusCode::BadRequest,
                        })?
                } else {
                    return Ok(request);
                };

                // read() doesn't return until an EOF signal is read; HTTP requests don't send EOF
                // terminators, so in order to read the body, we need to know the Content-Length
                let mut total_bytes_read: usize = request.body.len();
                while total_bytes_read < declared_total_bytes {
                    let mut buf: [u8; 512] = [0; 512];
                    match stream.read(&mut buf) {
                        Err(e) => {
                            return Err(RequestError {
                                status_code: HttpStatusCode::InternalServerError,
                            });
                        }
                        Ok(bytes_read) => {
                            if bytes_read == 0 {
                                return Err(RequestError {
                                    status_code: HttpStatusCode::BadRequest,
                                });
                            }
                            let bytes = &buf[0..bytes_read];
                            println!("{:#?}", bytes);
                            request.body.extend(bytes);
                            total_bytes_read += bytes_read;
                        }
                    }
                }
                Ok(request)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    mod fixtures {
        use crate::request::http_method::HttpMethod;

        pub fn create_headers(
            http_method: HttpMethod,
            content_length_override: Option<u64>,
            header_size_override: Option<usize>,
            omit_terminator: bool,
        ) -> String {
            let accept = "text/html,application/xhtml+xml,application/xml;q=0.9,/;q=0.8";
            let authorization = "Basic dXNlcjpwYXNz";

            let mut content_length = 0u64;

            if let Some(content_length_override) = content_length_override {
                content_length = content_length_override;
            };

            let content_type = "text/html";
            let host = "example.com";
            let user_agent = "ExampleBrowser/1.0";

            let mut headers: String = format!(
                "{} /path?foo=bar HTTP/1.1\r\n\
                Host: {}\r\n\
                Accept: {}\r\n\
                Authorization: {}\r\n\
                Content-Length: {}\r\n\
                Content-Type: {}\r\n\
                User-Agent: {}",
                http_method, host, accept, authorization, content_length, content_type, user_agent,
            );

            if let Some(desired_header_size) = header_size_override {
                let padding: String = format!(
                    "{}\r\nX-Pad: {}",
                    headers,
                    "A".repeat(desired_header_size - headers.len() - "X-Pad: ".len())
                );
                headers.push_str(&padding)
            };

            if omit_terminator {
                return headers;
            }
            headers.push_str("\r\n\r\n");
            headers
        }

        pub fn create_stream(headers: &String, mut body: Option<String>) -> String {
            if body.is_none() {
                body = Some(String::from("BODY_FIXTURE"));
            }
            return format!("{}{}", headers, body.unwrap());
        }
    }

    /**
     * Tests a scenario when the HTTP_HEADER_TERMINATOR arrives in sequential reads;
     * creates a stream with a header that is 510 bytes in size which results in
     * the terminator sequence \r\n\r\n being split across two reads
     */
    #[test]
    fn it_handles_header_terminator_across_sequential_reads() {
        let headers = fixtures::create_headers(HttpMethod::Get, None, Some(510), false);
        let stream = fixtures::create_stream(&headers, None);

        let request = Request::from_stream(&mut stream.as_bytes()).unwrap();
        let headers = request.headers;

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.request_target, "/path?foo=bar");
        assert_eq!(headers.host.unwrap(), "example.com");
        assert_eq!(
            headers.accept.unwrap(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,/;q=0.8"
        );
        assert_eq!(headers.user_agent.unwrap(), "ExampleBrowser/1.0");
        assert_eq!(headers.authorization.unwrap(), "Basic dXNlcjpwYXNz");
        assert_eq!(request.body, b"BODY_FIXTURE");
    }

    #[test]
    fn it_parses_request_line() {
        let headers = fixtures::create_headers(HttpMethod::Get, None, None, false);
        let stream = fixtures::create_stream(&headers, None);
        let request = Request::from_stream(&mut stream.as_bytes()).unwrap();
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.request_target, "/path?foo=bar");
    }

    #[test]
    fn it_handles_malformed_request_line() {
        let body = String::from("");
        let headers: String = String::from("POST   /path?foo=bar HTTP/1.1\r\n\r\n");
        let stream = fixtures::create_stream(&headers, Some(body.clone()));
        let error = RequestError {
            status_code: HttpStatusCode::BadRequest,
        };
        assert_eq!(Request::from_stream(&mut stream.as_bytes()), Err(error))
    }

    #[test]
    fn it_handles_method_not_implemented() {
        let headers: String = String::from("NOTIMPLEMENTED /path?foo=bar HTTP/1.1\r\n\r\n");
        let stream = fixtures::create_stream(&headers, None);
        let error = RequestError {
            status_code: HttpStatusCode::NotImplemented,
        };
        assert_eq!(Request::from_stream(&mut stream.as_bytes()), Err(error))
    }

    #[test]
    fn it_parses_header_fields() {
        let headers = fixtures::create_headers(HttpMethod::Get, None, None, false);
        let stream = fixtures::create_stream(&headers, None);

        let request = Request::from_stream(&mut stream.as_bytes()).unwrap();
        let headers = request.headers;

        assert_eq!(headers.host.unwrap(), "example.com");
        assert_eq!(
            headers.accept.unwrap(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,/;q=0.8"
        );
        assert_eq!(headers.user_agent.unwrap(), "ExampleBrowser/1.0");
        assert_eq!(headers.authorization.unwrap(), "Basic dXNlcjpwYXNz");
        assert_eq!(request.body, b"BODY_FIXTURE");
    }

    #[test]
    fn it_handles_missing_header_terminator() {
        let headers = fixtures::create_headers(HttpMethod::Get, None, None, true);
        let stream = fixtures::create_stream(&headers, None);

        let error = RequestError {
            status_code: HttpStatusCode::BadRequest,
        };
        assert_eq!(Request::from_stream(&mut stream.as_bytes()), Err(error))
    }

    #[test]
    fn it_handles_content_length_mismatch() {
        let body = String::from("");
        let headers = fixtures::create_headers(HttpMethod::Post, Some(512u64), None, false);
        let stream = fixtures::create_stream(&headers, Some(body.clone()));
        let error = RequestError {
            status_code: HttpStatusCode::BadRequest,
        };
        assert_eq!(Request::from_stream(&mut stream.as_bytes()), Err(error))
    }

    #[test]
    fn it_handles_content_length_type_mismatch() {
        let body = String::from("");
        let headers: String =
            String::from("POST /path?foo=bar HTTP/1.1\r\nContent-Length: abc\r\n\r\n");
        let stream = fixtures::create_stream(&headers, Some(body.clone()));
        let error = RequestError {
            status_code: HttpStatusCode::BadRequest,
        };
        assert_eq!(Request::from_stream(&mut stream.as_bytes()), Err(error))
    }

    #[test]

    fn it_handles_content_length_required() {
        let body = String::from("");
        let headers: String = String::from("POST /path?foo=bar HTTP/1.1\r\n\r\n");
        let stream = fixtures::create_stream(&headers, Some(body.clone()));
        let error = RequestError {
            status_code: HttpStatusCode::LengthRequired,
        };
        assert_eq!(Request::from_stream(&mut stream.as_bytes()), Err(error))
    }
}
