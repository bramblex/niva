use crate::{env::Config, thread_pool::ThreadPool};
use std::{
    io::{BufRead, Error, ErrorKind, Result, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{Arc, Mutex},
};

fn get_tcp_listener() -> Result<(TcpListener, u16)> {
    for port in 1025..65535 {
        match TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => {
                return Ok((l, port));
            }
            _ => {}
        }
    }
    return Err(Error::new(ErrorKind::Other, "No available port to listen"));
}

pub fn start(work_dir: &Path, config: &Config, thread_pool: &Arc<Mutex<ThreadPool>>) -> String {
    let entry = config.entry.clone().unwrap_or("index.html".to_string());
    let root_dir = work_dir.to_path_buf().clone();

    let (listener, port) = get_tcp_listener().unwrap();
    let thread_pool = thread_pool.clone();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let entry = entry.clone();
            let root_dir = root_dir.clone();
            match stream {
                Ok(mut stream) => {
                    thread_pool.lock().unwrap().run(move || {
                        let request_path = get_request_path(&mut stream);
                        let file_path = if request_path == "/" {
                            root_dir.join(entry)
                        } else {
                            root_dir.join(request_path.strip_prefix("/").unwrap())
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

    let entry_url = format!("http://127.0.0.1:{port}");
    println!("Server started at {}", entry_url);
    return entry_url;
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
