use std::{net::SocketAddr, sync::Arc};

use anyhow::{anyhow, Result};
use smol::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    stream::StreamExt,
};

use super::NivaResourceRef;

pub struct NivaResourceServer {
    pub port: u16,
    pub listener: Arc<TcpListener>,
}
pub type NivaResourceServerRef = Arc<NivaResourceServer>;

impl NivaResourceServer {
    pub async fn new() -> Result<Arc<NivaResourceServer>> {
        let (listener, port) = Self::get_tcp_listener().await?;
        let server = Arc::new(Self { port, listener });
        Ok(server)
    }

    pub async fn run(self: Arc<Self>, resource: NivaResourceRef) -> Result<()> {
        let mut incoming = self.listener.incoming();
        while let Some(stream) = incoming.next().await {
            if let Ok(stream) = stream {
                smol::spawn(Self::handle_request(stream, resource.clone())).detach();
            }
        }
        Ok(())
    }

    async fn handle_request(mut stream: TcpStream, resource: NivaResourceRef) -> Result<()> {
        let request_path = Self::get_request_path(&mut stream).await?;

        let resource = resource.clone();
        if let Ok(content) = resource.read_all(&request_path).await {
            Self::write_response(&mut stream, request_path, content).await?;
        } else {
            Self::write_404_response(&mut stream).await?;
        }
        Ok(())
    }

    async fn get_request_path(stream: &mut TcpStream) -> Result<String> {
        let buf_reader = BufReader::new(stream);
        let request_line = buf_reader
            .lines()
            .next()
            .await
            .ok_or(anyhow!("Unexpected request."))??;
        let request_path = request_line
            .split(' ')
            .collect::<Vec<&str>>()
            .get(1)
            .ok_or(anyhow!("Unexpected request."))?
            .to_string();
        let request_path = if request_path.ends_with('/') {
            request_path + "index.html"
        } else {
            request_path
        };
        Ok(request_path
            .strip_prefix("/")
            .ok_or(anyhow!("Unexpected request."))?
            .to_string())
    }

    async fn write_404_response(stream: &mut TcpStream) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        let error_message = b"404 Not Found";
        let len = error_message.len();
        buf.extend_from_slice(b"HTTP/1.1 404 NOT FOUND\r\n");
        buf.extend_from_slice(format!("Content-Length: {len}\r\n").as_bytes());
        buf.extend_from_slice(b"Content-Type: text/plain\r\n");
        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(error_message);
        stream.write_all(&buf).await?;
        Ok(())
    }

    async fn write_response(
        stream: &mut TcpStream,
        request_path: String,
        content: Vec<u8>,
    ) -> Result<()> {
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
        stream.write_all(&buf).await?;
        Ok(())
    }

    async fn get_tcp_listener() -> Result<(Arc<TcpListener>, u16)> {
        for port in 0xcfff..0xefff {
            if let Ok(listener) = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await
            {
                return Ok((Arc::new(listener), port));
            }
        }
        Err(anyhow!("No available port to listen."))
    }
}
