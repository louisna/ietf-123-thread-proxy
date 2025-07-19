//! This example is a modified version of:
//! https://github.com/hyperium/hyper/blob/0.14.x/examples/hello.rs

use bytes::Bytes;
use http_body_util::BodyExt;
use http_body_util::combinators::BoxBody;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{TcpListener, TcpStream};
type ClientBuilder = hyper::client::conn::http1::Builder;

async fn index1(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    // Here we do the proxy job.
    let host = req.uri().host().expect("uri has no host");
    let port = req.uri().port_u16().unwrap_or(80);

    let stream = TcpStream::connect((host, port)).await.unwrap();
    let io = TokioIo::new(stream);

    let (mut sender, conn) = ClientBuilder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(io)
        .await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let resp = sender.send_request(req).await?;
    Ok(resp.map(|b| b.boxed()))
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([10, 0, 0, 1], 3000).into();

    // Create the MPTCP capable socket but allow for a fallback to
    // TCP if the host does not support MPTCP.
    let socket = match Socket::new(
        Domain::for_address(addr),
        Type::STREAM,
        Some(Protocol::MPTCP),
    ) {
        Ok(socket) => socket,
        Err(err) => {
            eprintln!(
                "Unable to create an MPTCP socket, fallback to regular TCP socket: {}",
                err
            );
            Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?
        }
    };
    // Set common options on the socket as we created it by hand.
    socket.set_nonblocking(true)?;
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(1024)?;

    let listener = TcpListener::from_std(socket.into())?;
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(index1))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
