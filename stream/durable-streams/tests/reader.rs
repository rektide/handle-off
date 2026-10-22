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

#[test]
fn failed_flush_replays_and_oversize_does_not_advance() {
    let (url, server) = peer(vec![
        ("GET /stream?offset=-1 HTTP/1.1", "tail", "payload"),
        ("GET /stream?offset=-1 HTTP/1.1", "tail", "payload"),
        ("GET /stream?offset=tail HTTP/1.1", "new-tail", "oversized payload"),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let reader = FileReader::open(dir.path().join("reader")).unwrap();
    let binding = binding(&url, 8);
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
