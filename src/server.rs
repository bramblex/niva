use std::convert::Infallible;
use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use tokio::net::TcpListener;

async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
	Ok(Response::new(Full::new(Bytes::from("Hello Tauri Lite!"))))
}


#[tokio::main]
pub async fn start(
    entry_path: std::path::PathBuf,
    work_dir: std::path::PathBuf,
    _listener: std::net::TcpListener,
) {
    let listener= TcpListener::from_std(_listener).unwrap();

		loop {
			let (stream, _) = listener.accept().await?;

			tokio::task::spawn(async move {
					if let Err(err) = http1::Builder::new()
							.serve_connection(stream, service_fn(hello))
							.await
					{
							println!("Error serving connection: {:?}", err);
					}
			});
	}
}
