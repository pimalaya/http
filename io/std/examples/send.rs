use std::{
    io::{stderr, Read, Write},
    net::{SocketAddr, TcpListener},
    thread::{self, JoinHandle},
};

use http_lib::send::{self, Request};
use http_std::Handler;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    info!("init logger and HTTP server");
    init_logger();
    let (addr, server) = init_server();

    info!("init HTTP I/O handler");
    let mut handler = Handler::try_from(addr).unwrap();

    info!("send HTTP request using Send flow");
    let req = Request::new("GET", "/", "1.0").body("OK");
    let mut flow = send::Flow::new(req);

    let response = loop {
        match flow.next() {
            Ok(response) => break response,
            Err(send::Io::Send) => handler.send(&mut flow).unwrap(),
        }
    };

    let headers = String::from_utf8_lossy(&response.bytes[..response.body_start]);
    info!("response headers: {headers:?}",);

    let body = String::from_utf8_lossy(&response.bytes[response.body_start..]);
    info!("response body: {body:?}");

    server.join().unwrap();
}

fn init_logger() {
    let layer = fmt::layer().with_writer(stderr);
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    tracing_subscriber::registry()
        .with(layer)
        .with(filter)
        .init();
}

fn init_server() -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let host = addr.ip();
    let port = addr.port();

    info!(?host, port, "spawn HTTP server");

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut input = vec![0; 512];
        stream.read(&mut input).unwrap();
        stream
            .write(b"HTTP/1.0 200 OK\r\nDate: Sat, 08 Mar 2025 18:42:29 GMT\r\nContent-Length: 2\r\n\r\nOK")
            .unwrap();
    });

    (addr, handle)
}
