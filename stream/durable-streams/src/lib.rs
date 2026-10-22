//! Finite HTTP reads with explicit source binding and durable file checkpoints.
//! Output is flushed before checkpointing; interruption can replay output.

use anyhow::{Context, Result, bail, ensure};
use reqwest::{Method, Url, blocking::Client, header::HeaderMap};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

/// URL parsing is the only normalization. Paths remain URL paths, not file paths.
pub fn source_url(value: &str) -> Result<Url> {
    let url = Url::parse(value)?;
    ensure!(matches!(url.scheme(), "http" | "https"), "HTTP(S) URL required");
    ensure!(url.host_str().is_some(), "source needs a host");
    ensure!(url.username().is_empty() && url.password().is_none(), "URL credentials unsupported");
    ensure!(url.query().is_none() && url.fragment().is_none(), "source query/fragment unsupported");
    Ok(url)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub source_url: String,
    /// Operator-declared generation, NOT a server-verified identity.
    pub incarnation: String,
    pub content_type: String,
    pub max_bytes: u64,
    pub output_profile: String,
}

impl Binding {
    pub fn new(url: &str, incarnation: String, content_type: String, max_bytes: u64) -> Result<Self> {
        ensure!(!incarnation.trim().is_empty(), "incarnation must not be empty");
        ensure!(!content_type.trim().is_empty(), "content type must not be empty");
        ensure!(max_bytes > 0 && max_bytes < u64::MAX, "invalid response bound");
        Ok(Self {
            source_url: source_url(url)?.to_string(),
            incarnation,
            content_type,
            max_bytes,
            output_profile: "http-body-v1".into(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub version: u32,
    pub binding: Binding,
    pub next_offset: String,
}

/// A bounded, complete response, ready for application-owned output/commit ordering.
#[derive(Debug)]
pub struct Batch {
    pub body: Vec<u8>,
    pub next_offset: String,
    pub up_to_date: bool,
}

pub struct HttpStream {
    client: Client,
    url: Url,
}

impl HttpStream {
    pub fn new(url: &str) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            url: source_url(url)?,
        })
    }

    /// Mutations are sent once. An ambiguous transport failure is not retried.
    pub fn create(&self, content_type: &str, fork: Option<&str>, offset: Option<&str>) -> Result<HeaderMap> {
        ensure!(offset.is_none() || fork.is_some(), "fork offset needs fork source");
        let mut request = self.client.put(self.url.clone()).header("content-type", content_type);
        if let Some(source) = fork {
            request = request.header("Stream-Forked-From", source);
        }
        if let Some(offset) = offset {
            request = request.header("Stream-Fork-Offset", offset);
        }
        let response = request.send()?;
        ensure!(response.status().is_success(), "create HTTP {}", response.status());
        Ok(response.headers().clone())
    }

    pub fn append(&self, content_type: &str, body: Vec<u8>) -> Result<HeaderMap> {
        let response = self.client.post(self.url.clone()).header("content-type", content_type).body(body).send()?;
        ensure!(response.status().is_success(), "append HTTP {}", response.status());
        Ok(response.headers().clone())
    }

    pub fn head(&self) -> Result<HeaderMap> {
        let response = self.client.request(Method::HEAD, self.url.clone()).send()?;
        ensure!(response.status().is_success(), "head HTTP {}", response.status());
        Ok(response.headers().clone())
    }

    pub fn read(&self, offset: &str, content_type: &str, max_bytes: u64) -> Result<Batch> {
        ensure!(max_bytes > 0 && max_bytes < u64::MAX, "invalid response bound");
        ensure!(!offset.is_empty(), "empty offset");
        let response = self.client.get(self.url.clone()).query(&[("offset", offset)]).send()?;
        ensure!(response.status() == reqwest::StatusCode::OK, "read HTTP {}", response.status());
        let headers = response.headers();
        let actual_type = required_header(headers, "content-type")?;
        ensure!(media_type(&actual_type) == media_type(content_type), "content type changed: {actual_type}");
        let next_offset = required_header(headers, "stream-next-offset")?;
        ensure!(!next_offset.is_empty(), "empty next offset");
        let up_to_date = headers.get("stream-up-to-date").is_some_and(|v| v == "true");
        if let Some(length) = response.content_length() {
            ensure!(length <= max_bytes, "response exceeds {max_bytes} bytes; checkpoint unchanged");
        }
        let mut body = Vec::new();
        response.take(max_bytes + 1).read_to_end(&mut body)?;
        ensure!(body.len() as u64 <= max_bytes, "response exceeds {max_bytes} bytes; checkpoint unchanged");
        Ok(Batch { body, next_offset, up_to_date })
    }
}

fn media_type(value: &str) -> String {
    value.split(';').next().unwrap_or(value).trim().to_ascii_lowercase()
}

fn required_header(headers: &HeaderMap, name: &str) -> Result<String> {
    Ok(headers.get(name).with_context(|| format!("missing {name}"))?.to_str()?.to_owned())
}

/// Holds an advisory exclusive lock for the entire read/output/checkpoint operation.
/// All writers must use this API. The lock sidecar is deliberately retained.
pub struct FileReader {
    path: PathBuf,
    _lock: File,
}

impl FileReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut name = path.as_os_str().to_os_string();
        name.push(".lock");
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(name)?;
        fs2::FileExt::try_lock_exclusive(&lock).context("reader already in use")?;
        Ok(Self { path, _lock: lock })
    }

    pub fn status(&self) -> Result<Option<Checkpoint>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        ensure!(file.metadata()?.len() <= 65536, "checkpoint exceeds 64 KiB");
        let state: Checkpoint = serde_json::from_reader(file.take(65537))?;
        ensure!(state.version == 1, "unsupported checkpoint version {}", state.version);
        ensure!(!state.next_offset.is_empty(), "empty checkpoint offset");
        Ok(Some(state))
    }

    /// Explicit replacement, also usable to initialize a reader at a supplied offset.
    pub fn reset(&self, binding: Binding, offset: String) -> Result<Checkpoint> {
        ensure!(!offset.is_empty(), "empty reset offset");
        let state = Checkpoint {
            version: 1,
            binding,
            next_offset: offset,
        };
        self.save(&state)?;
        Ok(state)
    }

    pub fn read_into(&self, binding: &Binding, output: &mut impl Write) -> Result<Batch> {
        let state = self.status()?;
        if let Some(state) = &state {
            ensure!(
                &state.binding == binding,
                "checkpoint binding mismatch; inspect status and explicitly reset if intended"
            );
        }
        let offset = state.as_ref().map_or("-1", |state| state.next_offset.as_str());
        let batch = HttpStream::new(&binding.source_url)?.read(offset, &binding.content_type, binding.max_bytes)?;
        output.write_all(&batch.body).context("output failed; checkpoint unchanged")?;
        output.flush().context("output flush failed; checkpoint unchanged")?;
        self.save(&Checkpoint {
            version: 1,
            binding: binding.clone(),
            next_offset: batch.next_offset.clone(),
        })?;
        Ok(batch)
    }

    fn save(&self, state: &Checkpoint) -> Result<()> {
        let parent = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let bytes = serde_json::to_vec_pretty(state)?;
        if bytes.len() > 65536 {
            bail!("checkpoint exceeds 64 KiB");
        }
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.write_all(b"\n")?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|error| error.error)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}
