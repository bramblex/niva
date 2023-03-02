use super::thread_pool::ThreadPool;
use std::{
    io::{BufRead, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{Arc, Mutex},
};

fn listen_available_port() -> Option<(TcpListener, u16)> {
    for port in 1025..65535 {
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => {
                return Some((l, port));
            }
            _ => {}
        }
    }
    return None;
}

fn get_request_path(mut stream: &mut TcpStream) -> String {
    let buf_reader = std::io::BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();
    let request_path = request_line.split(" ").collect::<Vec<&str>>()[1];
    return request_path.to_string();
}

fn write_404_response(stream: &mut TcpStream) {
    let mut buf: Vec<u8> = Vec::new();
    let error_message = b"404 Not Found";
    let len = error_message.len();
    buf.extend_from_slice(b"HTTP/1.1 404 NOT FOUND\r\n");
    buf.extend_from_slice(format!("Content-Length: {len}\r\n").as_bytes());
    buf.extend_from_slice(b"Content-Type: text/plain\r\n");
    buf.extend_from_slice(b"\r\n");
    buf.extend_from_slice(error_message);
    stream.write_all(&buf).unwrap();
}

fn write_response(stream: &mut TcpStream, file_path: &Path, content: Vec<u8>) {
    let mut buf: Vec<u8> = Vec::new();

    let len = content.len();
    buf.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
    buf.extend_from_slice(format!("Content-Length: {len}\r\n").as_bytes());

    let mime_type = mime_guess::from_path(&file_path)
        .first()
        .unwrap_or(mime_guess::mime::TEXT_PLAIN)
        .to_string();
    buf.extend_from_slice(format!("Content-Type: {mime_type}\r\n").as_bytes());

    buf.extend_from_slice(b"\r\n");
    buf.extend_from_slice(&content);
    stream.write_all(&buf).unwrap();
}

pub fn start(
    thread_pool: Arc<Mutex<ThreadPool>>,
    entry_path: std::path::PathBuf,
    work_dir: std::path::PathBuf,
) -> String {
    let entry_path = entry_path.clone();
    let work_dir = work_dir.clone();
    let thread_pool = thread_pool.clone();

    let (listener, port) = listen_available_port().unwrap();

    let webview_url = "http://127.0.0.1:".to_string() + port.to_string().as_str();
    println!("Webview URL: {}", webview_url);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let entry_path = entry_path.clone();
            let work_dir = work_dir.clone();
            match stream {
                Ok(mut stream) => {
                    thread_pool.lock().unwrap().run(move || {
                        let request_path = get_request_path(&mut stream);
                        let file_path = if request_path == "/" {
                            entry_path.clone()
                        } else {
                            work_dir.join(request_path.strip_prefix("/").unwrap())
                        };
                        let file_result = std::fs::read(&file_path);
                        if file_result.is_err() {
                            write_404_response(&mut stream);
                            return;
                        }
                        write_response(&mut stream, &file_path, file_result.unwrap());
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
    });

    return webview_url;
}
