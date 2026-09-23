use std::{
    io::{self, BufRead, BufReader, Write},
    sync::{
        Arc, Mutex, Weak,
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
};

use anyhow::{Result, anyhow, bail};
use serde_json::{Map, Value, json};

use super::NivaApp;

const MAX_LINE_BYTES: usize = 64 * 1024 * 1024;
const OUTBOUND_QUEUE_CAPACITY: usize = 1;

/// NDJSON bridge used only when Niva was launched with `--stdio`.
pub(crate) struct StdioBridge {
    writer: SyncSender<Vec<u8>>,
    ready_sent: Mutex<bool>,
}

impl StdioBridge {
    pub(crate) fn start(app: Weak<NivaApp>) -> Result<Self> {
        let (writer, output) = mpsc::sync_channel::<Vec<u8>>(OUTBOUND_QUEUE_CAPACITY);
        let writer_app = app.clone();
        thread::Builder::new()
            .name("niva-stdio-writer".into())
            .spawn(move || {
                let stdout = io::stdout();
                let mut stdout = stdout.lock();
                while let Ok(frame) = output.recv() {
                    if let Err(error) = stdout.write_all(&frame).and_then(|()| stdout.flush()) {
                        eprintln!("[niva] stdio stdout write failed: {error}");
                        request_exit(&writer_app);
                        break;
                    }
                }
            })?;

        let input = Arc::new(Mutex::new(BufReader::new(io::stdin())));
        let reader = input.clone();
        let reader_app = app;
        smol::spawn(async move {
            loop {
                let reader = reader.clone();
                let result = smol::unblock(move || {
                    let mut reader = reader
                        .lock()
                        .map_err(|_| io::Error::other("stdin reader lock poisoned"))?;
                    read_line_limited(&mut *reader, MAX_LINE_BYTES)
                })
                .await;

                match result {
                    Ok(ReadLine::Line(line)) => handle_line(&reader_app, line),
                    Ok(ReadLine::TooLong) => {
                        eprintln!("[niva] stdio input line exceeds 64 MiB; discarded");
                    }
                    Ok(ReadLine::Eof) => {
                        request_exit(&reader_app);
                        break;
                    }
                    Err(error) => {
                        eprintln!("[niva] stdio stdin read failed: {error}");
                        request_exit(&reader_app);
                        break;
                    }
                }
            }
        })
        .detach();

        Ok(Self {
            writer,
            ready_sent: Mutex::new(false),
        })
    }

    /// Queue the one readiness frame after the main window's WebSocket hello.
    pub(crate) fn send_ready_once(&self) -> Result<()> {
        let mut ready_sent = self
            .ready_sent
            .lock()
            .map_err(|_| anyhow!("stdio ready state lock poisoned"))?;
        if *ready_sent {
            return Ok(());
        }

        let frame = encode_frame(&json!({ "t": "ready", "v": 1 }))?;
        match self.writer.try_send(frame) {
            Ok(()) => {
                *ready_sent = true;
                Ok(())
            }
            Err(TrySendError::Full(_)) => bail!("stdio output queue is full"),
            Err(TrySendError::Disconnected(_)) => bail!("stdio writer is unavailable"),
        }
    }

    pub(crate) fn send_message(&self, name: &str, data: Option<Value>) -> Result<()> {
        let mut frame = Map::new();
        frame.insert("t".into(), json!("msg"));
        frame.insert("name".into(), json!(name));
        if let Some(data) = data {
            frame.insert("data".into(), data);
        }
        let line = encode_frame(&Value::Object(frame))?;
        self.writer.try_send(line).map_err(|error| match error {
            TrySendError::Full(_) => anyhow!("stdio output queue is full"),
            TrySendError::Disconnected(_) => anyhow!("stdio writer is unavailable"),
        })
    }
}

fn encode_frame(frame: &Value) -> Result<Vec<u8>> {
    let mut line = serde_json::to_vec(frame)?;
    if line.len() + 1 > MAX_LINE_BYTES {
        bail!("stdio output line exceeds 64 MiB");
    }
    line.push(b'\n');
    Ok(line)
}

fn request_exit(app: &Weak<NivaApp>) {
    if let Some(app) = app.upgrade() {
        app.request_stdio_exit();
    }
}

fn handle_line(app: &Weak<NivaApp>, mut line: Vec<u8>) {
    if line.last() == Some(&b'\n') {
        line.pop();
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }

    let text = match std::str::from_utf8(&line) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("[niva] stdio input line is not UTF-8; discarded");
            return;
        }
    };
    let (name, data) = match parse_message(text) {
        Ok(message) => message,
        Err(error) => {
            eprintln!("[niva] invalid stdio input frame: {error}");
            return;
        }
    };

    let Some(app) = app.upgrade() else {
        return;
    };
    let window = match app.window().and_then(|windows| windows.get_window(0)) {
        Ok(window) => window,
        Err(_) => return,
    };

    let mut payload = Map::new();
    payload.insert("name".into(), json!(name));
    if let Some(data) = data {
        payload.insert("data".into(), data);
    }
    window.send_ipc_event("host:message", Value::Object(payload));
}

fn parse_message(text: &str) -> Result<(String, Option<Value>)> {
    let value: Value = serde_json::from_str(text)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("frame must be a JSON object"))?;
    if object.get("t").and_then(Value::as_str) != Some("msg") {
        bail!("unsupported frame type");
    }
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("msg frame requires a string name"))?
        .to_string();
    Ok((name, object.get("data").cloned()))
}

enum ReadLine {
    Line(Vec<u8>),
    TooLong,
    Eof,
}

/// Read at most `limit` bytes per line and drain an oversized line before
/// returning, so one bad frame cannot allocate without bound or poison the
/// following frame.
fn read_line_limited<R: BufRead>(reader: &mut R, limit: usize) -> io::Result<ReadLine> {
    let mut line = Vec::new();
    let mut too_long = false;

    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok(if too_long {
                ReadLine::TooLong
            } else if line.is_empty() {
                ReadLine::Eof
            } else {
                ReadLine::Line(line)
            });
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if !too_long {
            if count <= limit.saturating_sub(line.len()) {
                line.extend_from_slice(&available[..count]);
            } else {
                line.clear();
                too_long = true;
            }
        }
        reader.consume(count);

        if newline.is_some() {
            return Ok(if too_long {
                ReadLine::TooLong
            } else {
                ReadLine::Line(line)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use serde_json::{Value, json};

    use super::{ReadLine, parse_message, read_line_limited};

    #[test]
    fn reads_lf_and_crlf_lines_and_recovers_after_an_oversized_line() {
        let input = Cursor::new(b"abc\r\n123456789\nok\n".to_vec());
        let mut reader = BufReader::new(input);

        let ReadLine::Line(first) = read_line_limited(&mut reader, 8).unwrap() else {
            panic!("expected first line");
        };
        assert_eq!(first, b"abc\r\n");
        assert!(matches!(
            read_line_limited(&mut reader, 8).unwrap(),
            ReadLine::TooLong
        ));
        let ReadLine::Line(last) = read_line_limited(&mut reader, 8).unwrap() else {
            panic!("expected line after oversized input");
        };
        assert_eq!(last, b"ok\n");
        assert!(matches!(
            read_line_limited(&mut reader, 8).unwrap(),
            ReadLine::Eof
        ));
    }

    #[test]
    fn accepts_an_unterminated_final_line_at_eof() {
        let input = Cursor::new(b"{\"t\":\"msg\",\"name\":\"x\"}".to_vec());
        let mut reader = BufReader::new(input);
        let ReadLine::Line(line) = read_line_limited(&mut reader, 64).unwrap() else {
            panic!("expected final line");
        };
        assert_eq!(line, br#"{"t":"msg","name":"x"}"#);
        assert!(matches!(
            read_line_limited(&mut reader, 64).unwrap(),
            ReadLine::Eof
        ));
    }

    #[test]
    fn accepts_only_message_frames_and_preserves_explicit_null_data() {
        assert_eq!(
            parse_message(r#"{"t":"msg","name":"sayHello"}"#).unwrap(),
            ("sayHello".to_string(), None)
        );
        assert_eq!(
            parse_message(r#"{"t":"msg","name":"sayHello","data":null}"#).unwrap(),
            ("sayHello".to_string(), Some(Value::Null))
        );
        assert!(parse_message(r#"{"t":"ready","v":1}"#).is_err());
        assert!(parse_message(r#"{"t":"msg","data":1}"#).is_err());
    }

    #[test]
    fn parses_arbitrary_json_payloads() {
        let (name, data) =
            parse_message(r#"{"t":"msg","name":"sayHello","data":{"greeting":"hello","count":2}}"#)
                .unwrap();
        assert_eq!(name, "sayHello");
        assert_eq!(data, Some(json!({ "greeting": "hello", "count": 2 })));
    }
}
