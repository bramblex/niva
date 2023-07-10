use std::{net::SocketAddr, sync::Arc};

use anyhow::{anyhow, Result};
use smol::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    stream::StreamExt,
};

use super::NivaResourceRef;

pub struct NivaResourceServer {
    pub port: u16,
    pub listener: TcpListener,
    pub resource: NivaResourceRef,
}

impl NivaResourceServer {
    pub async fn new(resource: NivaResourceRef) -> Result<Arc<NivaResourceServer>> {
        let (listener, port) = Self::get_tcp_listener().await?;
        let server = Arc::new(Self {
            port,
            listener,
            resource,
        });
        smol::spawn(server.clone().run()).detach();
        println!("listen at 127.0.0.1: {}", port);
        Ok(server)
    }

    async fn get_tcp_listener() -> Result<(TcpListener, u16)> {
        for port in 1025..u16::MAX {
            if let Ok(listener) = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await
            {
                return Ok((listener, port));
            }
        }
        Err(anyhow!("No available port to listen."))
    }

    pub async fn run(self: Arc<Self>) -> Result<()> {
        let mut incoming = self.listener.incoming();
        while let Some(stream) = incoming.next().await {
            let mut stream = stream?;
            let buf_reader = BufReader::new(&mut stream);
            let request_line = buf_reader.lines().next().await.unwrap().unwrap();
            let request_path = request_line.split(' ').collect::<Vec<&str>>()[1].to_string();
            let request_path = request_path.strip_prefix("/").unwrap().to_string();
            let request_path = if request_path.ends_with('/') {
                request_path + "index.html"
            } else {
                request_path
            };
            let resource = self.resource.clone();
            if let Ok(content) = resource.read_all(&request_path).await {
                let mut buf: Vec<u8> = Vec::new();

                let len = content.len();
                buf.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
                buf.extend_from_slice(format!("Content-Length: {len}\r\n").as_bytes());

                let mime_type = mime_guess::from_path(request_path)
                    .first()
                    .unwrap_or(mime_guess::mime::TEXT_PLAIN)
                    .to_string();
                buf.extend_from_slice(format!("Content-Type: {mime_type}\r\n").as_bytes());

                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(&content);
                stream.write_all(&buf).await.unwrap();
            } else {
                let mut buf: Vec<u8> = Vec::new();
                let error_message = b"404 Not Found";
                let len = error_message.len();
                buf.extend_from_slice(b"HTTP/1.1 404 NOT FOUND\r\n");
                buf.extend_from_slice(format!("Content-Length: {len}\r\n").as_bytes());
                buf.extend_from_slice(b"Content-Type: text/plain\r\n");
                buf.extend_from_slice(b"\r\n");
                buf.extend_from_slice(error_message);
                stream.write_all(&buf).await.unwrap();
            }
        }
        Ok(())
    }
}
