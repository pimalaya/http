#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use std::{
    io::{self, Result},
    net::{SocketAddr, TcpStream},
};

use http_lib::send::{
    request::{CR, CRLF, LF},
    Response, State,
};
use memchr::memmem;
use tcp_lib::{read, write};
use tcp_std::Handler as TcpHandler;
use tracing::{instrument, warn};

const CRLF_CRLF: [u8; 4] = [CR, LF, CR, LF];
const CONTENT_LENGTH: &[u8] = b"Content-Length";

/// The standard, blocking I/O handler.
///
/// This handler makes use of the standard module [`std::http`]
/// to spawn httpes and wait for exit status or output.
#[derive(Debug)]
pub struct Handler {
    tcp: TcpHandler,
}

impl Handler {
    #[instrument("http/std/new", skip_all)]
    pub fn new(host: impl AsRef<str>, port: u16) -> Result<Self> {
        let tcp = TcpHandler::new(host, port)?;
        Ok(Self { tcp })
    }

    #[instrument(skip_all)]
    pub fn send(&mut self, mut flow: impl AsMut<State>) -> Result<()> {
        let state = flow.as_mut();
        let request = state.take_request();
        let mut response_bytes = Vec::new();
        let mut response_body_start = 0;

        let mut flow = write::Flow::new(request);

        while let Err(write::Io::Write) = flow.next() {
            self.tcp.write(&mut flow)?;
        }

        let mut flow = read::Flow::new();
        let mut response_body_length = 0;

        loop {
            match flow.next() {
                Ok(&[]) => {
                    break;
                }
                Ok(bytes) => {
                    response_bytes.extend(bytes);
                }
                Err(read::Io::Read) => {
                    self.tcp.read(&mut flow)?;
                    continue;
                }
            }

            if response_body_start == 0 {
                let body_start = memmem::find(&response_bytes, &CRLF_CRLF);

                if let Some(n) = body_start {
                    response_body_start = n + 4;
                }
            }

            if response_body_start > 0 && response_body_length == 0 {
                let mut content_length_start = None;

                for crlf in memmem::find_iter(&response_bytes, &CRLF) {
                    if let Some(start) = content_length_start {
                        let length = &response_bytes[start..crlf];
                        let length = String::from_utf8_lossy(length);
                        response_body_length = length.trim().parse().unwrap();
                        break;
                    }

                    // take bytes after the found CRLF
                    let crlf = crlf + CRLF.len();
                    let bytes = &response_bytes[crlf..];

                    // break if length of bytes after CRLF is
                    // smaller than `Content-Length: 0`
                    let colon_space_digit = 3;
                    if bytes.len() < CONTENT_LENGTH.len() + colon_space_digit {
                        break;
                    }

                    // search for another CRLF if header does
                    // not match Content-Type
                    if !bytes[..CONTENT_LENGTH.len()].eq_ignore_ascii_case(CONTENT_LENGTH) {
                        continue;
                    }

                    content_length_start = Some(crlf + CONTENT_LENGTH.len() + 1);
                }
            }

            if response_body_start > 0 && response_body_length > 0 {
                let body_bytes = &response_bytes[response_body_start..];

                if body_bytes.len() >= response_body_length {
                    break;
                }
            }
        }

        state.set_response(Response {
            bytes: response_bytes,
            body_start: response_body_start,
        });

        Ok(())
    }
}

impl From<TcpHandler> for Handler {
    fn from(tcp: TcpHandler) -> Self {
        Self { tcp }
    }
}

impl From<TcpStream> for Handler {
    fn from(stream: TcpStream) -> Self {
        let tcp = TcpHandler::from(stream);
        Self { tcp }
    }
}

impl TryFrom<SocketAddr> for Handler {
    type Error = io::Error;

    fn try_from(addr: SocketAddr) -> io::Result<Self> {
        let tcp = TcpHandler::try_from(addr)?;
        Ok(Self { tcp })
    }
}
