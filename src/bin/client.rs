use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use std::env;
use std::net::SocketAddr;
use hyper::Request;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::io::{self, AsyncWriteExt as _};
use tokio::net::TcpStream;
use hyper_util::rt::TokioIo;


// A simple type alias so as to DRY.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {

    // Some simple CLI args requirements...
    let url = match env::args().nth(1) {
        Some(url) => url,
        None => {
            println!("Usage: client <url>");
            return Ok(());
        }
    };

    // HTTPS requires picking a TLS implementation, so give a better
    // warning if the user tries to request an 'https' URL.
    let url = url.parse::<hyper::Uri>().unwrap();
    if url.scheme_str() != Some("http") {
        println!("This example only works with 'http' URLs.");
        return Ok(());
    }

    fetch_url(url).await
}

async fn fetch_url(url: hyper::Uri) -> Result<()> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(3000);
    let addr_str = format!("{}:{}", host, port);
    let addr: SocketAddr = addr_str.parse().unwrap();
    let addr_socket2 = SockAddr::from(addr);

    let local_addr = "0.0.0.0:0".parse().unwrap();
    let socket = Socket::new(
        Domain::for_address(local_addr),
        Type::STREAM,
        Some(Protocol::MPTCP),
    ).unwrap();

    socket.set_reuse_address(true)?;
    socket.bind(&local_addr.into())?;
    socket.connect(&addr_socket2)?;
    socket.set_nonblocking(true)?;

    let stream = TcpStream::from_std(socket.into())?;
    let io = TokioIo::new(stream);

    let (mut request_sender, conn) = hyper::client::conn::http1::Builder::new().handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let authority = url.authority().unwrap().clone();

    let path = url.path();
    let req = Request::builder()
        .uri(path)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let mut res = request_sender.send_request(req).await?;

    println!("Response: {}", res.status());
    println!("Headers: {:#?}\n", res.headers());

    // Stream the body, writing each chunk to stdout as we get it
    // (instead of buffering and printing at the end).
    while let Some(next) = res.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            io::stdout().write_all(chunk).await?;
        }
    }

    println!("\n\nDone!");

    Ok(())
}