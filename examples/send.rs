use std::{
    env,
    io::{stdin, stdout, ErrorKind, Read, Write},
    net::TcpStream,
    sync::Arc,
};

use http_flows::{flows::Send, Request};
use log::warn;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use rustls_platform_verifier::ConfigVerifierExt;
use stream_flows::handlers::std::Handler;
use url::Url;

fn main() {
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "debug");
    }

    env_logger::init();

    let url: Url = match env::var("URL") {
        Ok(url) => url.parse().unwrap(),
        Err(_) => read_line("URL?").parse().unwrap(),
    };

    let mut stream = connect(&url);

    let request = Request::new("GET", url.path(), "1.0").body("");
    let mut send = Send::new(request);

    let response = loop {
        match send.next() {
            Ok(response) => {
                break response;
            }
            Err(io) => {
                if let Err(err) = Handler::handle(&mut stream, &mut send, io) {
                    if err.kind() == ErrorKind::UnexpectedEof {
                        warn!("unexpected eof: {err}");
                        send.as_mut().set_written_bytes_count(0);
                    }
                }
            }
        }
    };

    println!("----------------");
    let headers = String::from_utf8_lossy(&response.bytes[..response.body_start]);
    println!("{}", headers.trim());
    println!("----------------");
    let body = String::from_utf8_lossy(&response.bytes[response.body_start..]);
    println!("{}", body.trim());
    println!("----------------");
}

fn read_line(prompt: &str) -> String {
    print!("{prompt} ");
    stdout().flush().unwrap();

    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();

    line.trim().to_owned()
}

trait StreamExt: Read + Write {}
impl<T: Read + Write> StreamExt for T {}

fn connect(url: &Url) -> Box<dyn StreamExt> {
    let domain = url.domain().unwrap();
    if url.scheme().eq_ignore_ascii_case("https") {
        let config = ClientConfig::with_platform_verifier();
        let server_name = domain.to_string().try_into().unwrap();
        let conn = ClientConnection::new(Arc::new(config), server_name).unwrap();
        let tcp = TcpStream::connect((domain.to_string(), 443)).unwrap();
        let tls = StreamOwned::new(conn, tcp);
        Box::new(tls)
    } else {
        let tcp = TcpStream::connect((domain.to_string(), 80)).unwrap();
        Box::new(tcp)
    }
}
