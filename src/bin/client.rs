use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use std::net::SocketAddr;
use hyper::Request;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use tokio::net::TcpStream;
use hyper_util::rt::TokioIo;
use std::process::Command;
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// URL of the server.
    url: String,

    /// Whether the client will put its link to sleep.
    #[clap(long)]
    sleep: bool
}

// A simple type alias so as to DRY.
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // HTTPS requires picking a TLS implementation, so give a better
    // warning if the user tries to request an 'https' URL.
    let url = args.url.parse::<hyper::Uri>().unwrap();
    if url.scheme_str() != Some("http") {
        println!("This example only works with 'http' URLs.");
        return Ok(());
    }

    fetch_url(url, args.sleep).await
}

async fn fetch_url(url: hyper::Uri, sleep: bool) -> Result<()> {
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

    // Add primary if the previous test failed.
    Command::new("./add_primary.sh").spawn()?.wait()?;

    Command::new("./add_backup.sh").spawn()?.wait()?;

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

    // First request to establish the second MPTCP path.
    let req = Request::builder().uri("/sleep")
            .header(hyper::header::HOST, authority.as_str())
            .body(Empty::<Bytes>::new())?;

    let res = request_sender.send_request(req).await?;
    if res.status() != 200 {
        return Err("Impossible to sleep".to_string().into());
    }    

    tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;

    let path = url.path();
    let req = Request::builder()
        .uri(path)
        .header(hyper::header::HOST, authority.as_str())
        .body(Empty::<Bytes>::new())?;

    let res = request_sender.send_request(req);

    if sleep {
        println!("Killing the first interface");
        Command::new("./kill_primary.sh").spawn().unwrap().wait().unwrap();
    }

    println!("Before waiting for the response");
    let mut res = res.await?;
    println!("After waiting for the response");
    
    println!("Response: {}", res.status());
    println!("Headers: {:#?}\n", res.headers());

    if sleep {
        // We received the response over the other path. Activate again the WiFi interface.
        Command::new("./add_primary.sh").spawn()?.wait()?;
    }

    // Stream the body, writing each chunk to stdout as we get it
    // (instead of buffering and printing at the end).
    let mut read = 0;
    while let Some(next) = res.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            // io::stdout().write_all(chunk).await?;
            read += chunk.len();
        }
    }

    println!("\n\nDone! Read {read} bytes");

    Ok(())
}