use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, mpsc::Sender};
use std::thread;

use crate::request::Request;
use crate::response::Response;
use crate::router::Router;

/**
 * A shared handler that handles the stream.
 *
 * The default handler closes over a Router
 * and passes the stream into it so it can be
 * handled by the appropriate route handler.
 */
type StreamHandler = Arc<dyn Fn(TcpStream) + Send + Sync>;

pub struct ServerOptions {
    stream_handler: Option<StreamHandler>,
    channel_message_sender: Option<Sender<()>>,
}
pub struct Server {}

impl Server {
    pub fn start(router: Router, port: u16, options: Option<ServerOptions>) {
        let stream_handler: StreamHandler;
        let mut sender: Option<Sender<()>> = None;
        match options {
            Some(server_options) => {
                stream_handler = server_options
                    .stream_handler
                    .unwrap_or(Self::create_stream_handler(router));
                sender = server_options.channel_message_sender;
            }
            None => {
                stream_handler = Self::create_stream_handler(router);
            }
        }
        Server::bind(stream_handler, sender, port);
    }

    // Bind to a port and spawn threads to handle incoming connections.
    // Each thread clones the Dispatcher, which is an Arc-wrapped function
    // that routes the stream to the proper route handler
    fn bind(stream_handler: StreamHandler, sender: Option<Sender<()>>, port: u16) {
        match TcpListener::bind(format!("127.0.0.1:{port}")) {
            Err(e) => println!("Error binding to port {port}: {:?}", e),
            Ok(tcp_listener) => {
                if let Some(sender) = sender {
                    sender
                        .send(())
                        .expect("Error sending server started signal");
                };

                for stream in tcp_listener.incoming() {
                    match stream {
                        Err(e) => println!("Error with stream: {:?}", e),
                        Ok(stream) => {
                            let threaded_stream_handler = stream_handler.clone();
                            // thread::spawn requires ownership but the closure doesn't know that.
                            // We need to explicitly specify `move` capture mode
                            thread::spawn(move || threaded_stream_handler(stream));
                        }
                    }
                }
            }
        };
    }

    fn create_stream_handler(router: Router) -> StreamHandler {
        Arc::new(move |mut stream: TcpStream| {
            let response: Response = match Request::from_stream(&mut stream) {
                Err(e) => Response::new(e.status_code, e.message().into()),
                Ok(request) => {
                    println!(
                        "[Router] [{:?}] {:?}",
                        request.method(),
                        request.request_target()
                    );
                    router.dispatch(request)
                }
            };
            stream.write_all(&response.to_bytes()).unwrap(); //FIXME error_handling
        })
    }
}

#[cfg(test)]
mod test {
    use std::io::{Error, Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::sync::mpsc::channel;
    use std::time::{Duration, Instant};
    use std::{assert_eq, panic};

    use super::*;

    const TEST_HANDLER_SLEEP_TIME_MS: u16 = 500;
    const THREAD_OVERHEAD_BUFFER_MS: u16 = 100;
    const EXPECTED_DURATION_THRESHOLD_MS: u16 =
        TEST_HANDLER_SLEEP_TIME_MS + THREAD_OVERHEAD_BUFFER_MS;

    mod fixtures {
        use super::*;

        pub fn send_request(
            port: u16,
            hangup: bool,
            sender: Option<Sender<()>>,
        ) -> impl FnOnce() -> Result<[u8; 512], Error> {
            move || {
                let mut stream = TcpStream::connect(format!("localhost:{}", port))
                    .expect("Error connecting to localhost on port 6113");
                stream.write(b"ping").expect("Error sending request");

                if hangup {
                    stream.shutdown(Shutdown::Write).unwrap();
                }

                let mut response = [0; 512];
                if let Err(e) = stream.read(&mut response) {
                    return Err(e);
                }

                if let Some(sender) = sender {
                    if let Err(e) = sender.send(()) {
                        println!("{:?}", e);
                        panic!()
                    }
                }

                Ok(response)
            }
        }

        pub fn start_server(port: u16, stream_handler: Option<StreamHandler>) {
            let (sender, receiver) = channel::<()>();

            let options = ServerOptions {
                stream_handler,
                channel_message_sender: Some(sender),
            };
            let router = Router::new();

            thread::spawn(move || {
                Server::start(router, port, Some(options));
            });

            if let Err(e) = receiver.recv() {
                panic!("Error receiving server started signal: {:?}", e)
            }
        }
    }

    #[test]
    fn it_handles_requests_concurrently() {
        fn sleeping_stream_handler_fixture(mut stream: TcpStream) {
            thread::sleep(Duration::from_millis(TEST_HANDLER_SLEEP_TIME_MS.into()));
            stream.write(b"done").unwrap();
        }
        let stream_handler = Arc::new(sleeping_stream_handler_fixture);

        let port: u16 = 6113;
        fixtures::start_server(port, Some(stream_handler));

        let (sender, receiver) = channel::<()>();
        let sender_clone = sender.clone();

        let start = Instant::now();

        thread::spawn(fixtures::send_request(port, false, Some(sender)));
        thread::spawn(fixtures::send_request(port, false, Some(sender_clone)));

        receiver.recv().and_then(|_| receiver.recv()).unwrap();
        let duration = start.elapsed();
        assert!(duration.as_millis() < EXPECTED_DURATION_THRESHOLD_MS.into());
    }

    #[test]
    fn it_handles_client_disconnect() {
        let port: u16 = 6114;
        fixtures::start_server(port, None);
        let (sender, receiver) = channel::<()>();
        let thread = thread::spawn(fixtures::send_request(port, true, Some(sender)));
        receiver.recv_timeout(Duration::new(1, 0)).unwrap();
        let result = thread.join().unwrap();
        match result {
            Ok(response) => assert_eq!(
                "HTTP/1.1 400 Bad Request",
                std::str::from_utf8(&response)
                    .unwrap()
                    .split("\r\n\r\n")
                    .next()
                    .unwrap()
            ),
            Err(_) => panic!(),
        }
    }
}
