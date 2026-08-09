use crate::config::Config;
use crate::protocol::{Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub fn send(request: Request) -> Response {
    let socket_path = Config::socket_path();

    match UnixStream::connect(&socket_path) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(Duration::from_secs(3))).ok();

            let json = serde_json::to_string(&request).unwrap();
            if writeln!(stream, "{}", json).is_err() {
                return Response::DaemonNotRunning;
            }

            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                return Response::DaemonNotRunning;
            }

            serde_json::from_str(line.trim()).unwrap_or(Response::Error {
                message: "Failed to parse daemon response.".to_string(),
            })
        }
        Err(_) => Response::DaemonNotRunning,
    }
}

pub fn daemon_running() -> bool {
    Config::socket_path().exists()
}
