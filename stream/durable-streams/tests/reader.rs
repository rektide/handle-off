use handle_off_stream_durable::{Binding, FileReader, HttpStream, source_url};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

// A finite scripted HTTP peer: exact request targets make offset encoding observable.
fn peer(steps: Vec<(&'static str, &'static str, &'static str)>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/stream", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        for (request_line, offset, body) in steps {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
            let mut data = Vec::new();
            while !data.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                data.push(byte[0]);
            }
            let headers = String::from_utf8(data).unwrap();
            assert_eq!(headers.lines().next().unwrap(), request_line);
            let length = headers
                .lines()
                .find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.parse::<usize>().unwrap()))
                .unwrap_or(0);
            let mut input = vec![0; length];
            stream.read_exact(&mut input).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nStream-Next-Offset: {offset}\r\nStream-Up-To-Date: true\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    (url, handle)
}

fn binding(url: &str, max: u64) -> Binding {
    Binding::new(url, "generation-1".into(), "application/octet-stream".into(), max).unwrap()
}

#[test]
fn create_append_resume_and_two_readers() {
    let (url, server) = peer(vec![
        ("PUT /stream HTTP/1.1", "start", ""),
        ("POST /stream HTTP/1.1", "opaque:+/1", ""),
        ("HEAD /stream HTTP/1.1", "opaque:+/1", ""),
        ("GET /stream?offset=-1 HTTP/1.1", "opaque:+/1", "first"),
        ("GET /stream?offset=opaque%3A%2B%2F1 HTTP/1.1", "opaque:2", "second"),
        ("GET /stream?offset=-1 HTTP/1.1", "opaque:2", "firstsecond"),
    ]);
    let http = HttpStream::new(&url).unwrap();
    http.create("application/octet-stream", None, None).unwrap();
    http.append("application/octet-stream", b"first".to_vec()).unwrap();
    http.head().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.json");
    let b = dir.path().join("b.json");
    let mut output = Vec::new();
    FileReader::open(&a).unwrap().read_into(&binding(&url, 100), &mut output).unwrap();
    assert_eq!(output, b"first");
    output.clear();
    FileReader::open(&a).unwrap().read_into(&binding(&url, 100), &mut output).unwrap();
    assert_eq!(output, b"second");
    output.clear();
    FileReader::open(&b).unwrap().read_into(&binding(&url, 100), &mut output).unwrap();
    assert_eq!(output, b"firstsecond");
    assert_eq!(FileReader::open(&a).unwrap().status().unwrap().unwrap().next_offset, "opaque:2");
    server.join().unwrap();
}

struct FlushFailure(Vec<u8>);
impl Write for FlushFailure {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("fixture flush failure"))
    }
}

struct WriteFailure;
impl Write for WriteFailure {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("fixture write failure"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        panic!("flush must not follow failed write")
    }
}

#[test]
fn failed_flush_replays_and_oversize_does_not_advance() {
    let (url, server) = peer(vec![
        ("GET /stream?offset=-1 HTTP/1.1", "tail", "payload"),
        ("GET /stream?offset=-1 HTTP/1.1", "tail", "payload"),
        ("GET /stream?offset=-1 HTTP/1.1", "tail", "payload"),
        ("GET /stream?offset=tail HTTP/1.1", "new-tail", "oversized payload"),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let reader = FileReader::open(dir.path().join("reader")).unwrap();
    let binding = binding(&url, 8);
    assert!(reader.read_into(&binding, &mut WriteFailure).is_err());
    assert!(reader.status().unwrap().is_none());
    let mut failed = FlushFailure(Vec::new());
    assert!(reader.read_into(&binding, &mut failed).is_err());
    assert_eq!(failed.0, b"payload");
    assert!(reader.status().unwrap().is_none());
    let mut output = Vec::new();
    reader.read_into(&binding, &mut output).unwrap();
    assert_eq!(output, b"payload");
    output.clear();
    let before = reader.status().unwrap();
    assert!(reader.read_into(&binding, &mut output).is_err());
    assert!(output.is_empty());
    assert_eq!(reader.status().unwrap(), before);
    server.join().unwrap();
}

#[test]
fn binding_lock_corruption_and_reset() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state");
    let reader = FileReader::open(&path).unwrap();
    assert!(FileReader::open(&path).is_err());
    let original = binding("http://127.0.0.1:1/stream", 100);
    reader.reset(original.clone(), "opaque-offset".into()).unwrap();
    for changed in [
        Binding {
            source_url: "http://127.0.0.1:1/other".into(),
            ..original.clone()
        },
        Binding {
            incarnation: "generation-2".into(),
            ..original.clone()
        },
        Binding {
            max_bytes: 101,
            ..original.clone()
        },
        Binding {
            content_type: "text/plain".into(),
            ..original.clone()
        },
        Binding {
            output_profile: "other".into(),
            ..original.clone()
        },
    ] {
        let error = reader.read_into(&changed, &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("binding mismatch"));
    }
    std::fs::write(&path, "partial JSON").unwrap();
    assert!(reader.status().is_err());
    reader.reset(original, "-1".into()).unwrap();
    assert_eq!(reader.status().unwrap().unwrap().next_offset, "-1");
}

#[test]
fn reject_ambiguous_locators() {
    for url in ["file:///stream", "http://a/b?q=1", "http://a/b#x", "http://user:pass@a/b"] {
        assert!(source_url(url).is_err());
    }
    assert_eq!(source_url("HTTP://LOCALHOST:80/a%2Fb").unwrap().as_str(), "http://localhost/a%2Fb");
}

fn raw_peer(response: &'static [u8]) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/stream", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        stream.write_all(response).unwrap();
    });
    (url, handle)
}

#[test]
fn protocol_failures_never_output_or_checkpoint() {
    for response in [
        &b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n"[..],
        &b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/other\r\nContent-Length: 0\r\n\r\n"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nStream-Next-Offset: end\r\nContent-Length: 0\r\n\r\n"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 0\r\n\r\n"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nStream-Next-Offset: end\r\nContent-Length: 5\r\n\r\nabc"[..],
        &b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nStream-Next-Offset: end\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n12345\r\n5\r\n67890\r\n0\r\n\r\n"[..],
    ] {
        let (url, server) = raw_peer(response);
        let dir = tempfile::tempdir().unwrap();
        let reader = FileReader::open(dir.path().join("state")).unwrap();
        let mut output = Vec::new();
        assert!(reader.read_into(&binding(&url, 8), &mut output).is_err());
        assert!(output.is_empty());
        assert!(reader.status().unwrap().is_none());
        server.join().unwrap();
    }
}

#[test]
fn unsupported_and_oversized_checkpoint_config_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state");
    let reader = FileReader::open(&path).unwrap();
    let original = binding("http://127.0.0.1:1/stream", 100);
    let changed = Binding {
        output_profile: "future-v2".into(),
        ..original.clone()
    };
    assert!(reader.reset(changed, "-1".into()).is_err());
    assert!(reader.reset(original.clone(), "x".repeat(65536)).is_err());
    assert!(!path.exists());
    let saved = reader.reset(original, "-1".into()).unwrap();
    let mut state = serde_json::to_value(saved).unwrap();
    state["version"] = 2.into();
    std::fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(reader.status().unwrap_err().to_string().contains("unsupported checkpoint version"));
}

#[test]
fn installed_cli_reset_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state");
    let binary = env!("CARGO_BIN_EXE_durable-workbench");
    let reset = std::process::Command::new(binary)
        .args(["reset", "http://127.0.0.1:1/stream", "--incarnation", "generation-1", "--state"])
        .arg(&path)
        .args(["--offset", "opaque:+/1"])
        .output()
        .unwrap();
    assert!(reset.status.success(), "{}", String::from_utf8_lossy(&reset.stderr));
    let status = std::process::Command::new(binary).args(["status", "--state"]).arg(&path).output().unwrap();
    assert!(status.status.success());
    let state: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(state["next_offset"], "opaque:+/1");
}

#[test]
fn save_failure_after_output_keeps_old_offset_and_replays() {
    struct MoveParentOnFlush {
        original: std::path::PathBuf,
        moved: std::path::PathBuf,
        bytes: Vec<u8>,
    }
    impl Write for MoveParentOnFlush {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            std::fs::rename(&self.original, &self.moved)
        }
    }
    let (url, server) = peer(vec![
        ("GET /stream?offset=start HTTP/1.1", "tail", "payload"),
        ("GET /stream?offset=start HTTP/1.1", "tail", "payload"),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original");
    let moved = dir.path().join("moved");
    std::fs::create_dir(&original).unwrap();
    let binding = binding(&url, 100);
    let reader = FileReader::open(original.join("state")).unwrap();
    reader.reset(binding.clone(), "start".into()).unwrap();
    let mut writer = MoveParentOnFlush {
        original,
        moved: moved.clone(),
        bytes: Vec::new(),
    };
    let error = reader.read_into(&binding, &mut writer).unwrap_err();
    assert!(error.to_string().contains("output flushed but checkpoint save failed"));
    assert_eq!(writer.bytes, b"payload");
    drop(reader);
    let reopened = FileReader::open(moved.join("state")).unwrap();
    assert_eq!(reopened.status().unwrap().unwrap().next_offset, "start");
    let mut output = Vec::new();
    reopened.read_into(&binding, &mut output).unwrap();
    assert_eq!(output, b"payload");
    assert_eq!(reopened.status().unwrap().unwrap().next_offset, "tail");
    server.join().unwrap();
}
