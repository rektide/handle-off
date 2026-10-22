//! Pre-alpha source-bound journal positions. No execution authority is transferred.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

/// Source descriptions and checkpoints are small control documents, not journal payloads.
pub const MAX_STATE_BYTES: usize = 64 * 1024;

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let mut bytes = Vec::new();
    File::open(path)?.take(MAX_STATE_BYTES as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= MAX_STATE_BYTES, "control document exceeds 64 KiB");
    Ok(serde_json::from_slice(&bytes)?)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Backend {
    Fixture { path: PathBuf },
    Journal { scope: Scope, directory: Option<PathBuf> },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    System,
    User,
}

/// Select service-process records or manager records naming a target unit.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitField {
    SystemdUnit,
    UserUnit,
    ManagerTarget,
    UserManagerTarget,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Selection {
    FollowUnit,
    Invocation { id: String },
}

/// Explicit initial read policy; it has no effect once a cursor is committed.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum InitialPosition {
    Head,
    Tail,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub backend: Backend,
    pub machine_id: String,
    pub unit: String,
    pub unit_field: UnitField,
    pub selection: Selection,
    /// ANDed exact journal field matches; e.g. JOB_ID or an application task ID.
    #[serde(default)]
    pub matches: BTreeMap<String, String>,
}

fn id_valid(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

impl Source {
    pub fn load(path: &Path) -> Result<Self> {
        let source: Self = load_json(path)?;
        source.validate()?;
        Ok(source)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(id_valid(&self.machine_id), "machine_id must be 32 lowercase hex digits");
        ensure!(!self.unit.is_empty() && !self.unit.chars().any(char::is_control), "invalid unit");
        if let Selection::Invocation { id } = &self.selection {
            ensure!(id_valid(id), "invocation id must be 32 lowercase hex digits");
        }
        let path = match &self.backend {
            Backend::Fixture { path } => Some(path),
            Backend::Journal { directory, .. } => directory.as_ref(),
        };
        if let Some(path) = path {
            ensure!(path.is_absolute(), "source path must be absolute");
        }
        for (key, value) in &self.matches {
            ensure!(
                !key.is_empty() && key.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == b'_'),
                "invalid journal field {key}"
            );
            ensure!(
                !key.starts_with("__")
                    && ![
                        "_MACHINE_ID",
                        "_SYSTEMD_UNIT",
                        "_SYSTEMD_USER_UNIT",
                        "_SYSTEMD_INVOCATION_ID",
                        "UNIT",
                        "INVOCATION_ID",
                        "USER_INVOCATION_ID",
                    ]
                    .contains(&key.as_str()),
                "reserved selector {key}"
            );
            ensure!(!value.contains('\0'), "NUL in selector");
        }
        Ok(())
    }

    pub fn selectors(&self) -> BTreeMap<String, String> {
        let mut result = self.matches.clone();
        result.insert("_MACHINE_ID".into(), self.machine_id.clone());
        let unit_key = match self.unit_field {
            UnitField::SystemdUnit => "_SYSTEMD_UNIT",
            UnitField::UserUnit => "_SYSTEMD_USER_UNIT",
            UnitField::ManagerTarget | UnitField::UserManagerTarget => "UNIT",
        };
        result.insert(unit_key.into(), self.unit.clone());
        if let Selection::Invocation { id } = &self.selection {
            let key = match self.unit_field {
                UnitField::SystemdUnit | UnitField::UserUnit => "_SYSTEMD_INVOCATION_ID",
                UnitField::ManagerTarget => "INVOCATION_ID",
                UnitField::UserManagerTarget => "USER_INVOCATION_ID",
            };
            result.insert(key.into(), id.clone());
        }
        result
    }

    pub fn accepts(&self, event: &Event) -> bool {
        self.selectors().iter().all(|(key, wanted)| match event.fields.get(key) {
            Some(Value::String(value)) => value == wanted,
            Some(Value::Array(values)) => values.iter().any(|v| v.as_str() == Some(wanted)),
            _ => false,
        })
    }

    /// Describe the explicit adapter/selection without opening the source or state.
    pub fn describe(&self) -> Result<SourceDescription> {
        self.validate()?;
        Ok(SourceDescription {
            backend_profile: match self.backend {
                Backend::Fixture { .. } => "journal-jsonl-v1",
                Backend::Journal { .. } => "journalctl-json-v1",
            },
            source: self.clone(),
            selectors: self.selectors(),
            position_semantics: "exclusive-after-last-emitted-entry",
            cursor_comparison: "exact-string-only",
            output_profile: "journal-observation-jsonl-v1",
        })
    }
}

/// Adapter-specific description, not a registry, checkpoint converter or access probe.
#[derive(Clone, Debug, Serialize)]
pub struct SourceDescription {
    pub backend_profile: &'static str,
    pub source: Source,
    pub selectors: BTreeMap<String, String>,
    pub position_semantics: &'static str,
    pub cursor_comparison: &'static str,
    pub output_profile: &'static str,
}

/// JSON values are retained verbatim, including binary byte arrays and repeated fields.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Event {
    pub fields: Map<String, Value>,
}

impl Event {
    pub fn text(&self, key: &str) -> Option<&str> {
        self.fields.get(key)?.as_str()
    }
    pub fn cursor(&self) -> Result<&str> {
        let cursor = self.text("__CURSOR").context("missing/scalar-invalid __CURSOR")?;
        ensure!(!cursor.is_empty() && !cursor.chars().any(char::is_control), "invalid cursor");
        Ok(cursor)
    }
    pub fn identity(&self, source: &Source) -> Identity {
        let get = |key| self.text(key).map(str::to_owned);
        let unit_key = match source.unit_field {
            UnitField::SystemdUnit => "_SYSTEMD_UNIT",
            UnitField::UserUnit => "_SYSTEMD_USER_UNIT",
            UnitField::ManagerTarget | UnitField::UserManagerTarget => "UNIT",
        };
        let invocation_key = match source.unit_field {
            UnitField::SystemdUnit | UnitField::UserUnit => "_SYSTEMD_INVOCATION_ID",
            UnitField::ManagerTarget => "INVOCATION_ID",
            UnitField::UserManagerTarget => "USER_INVOCATION_ID",
        };
        Identity {
            machine_id: get("_MACHINE_ID"),
            boot_id: get("_BOOT_ID"),
            unit: get(unit_key),
            emitter_unit: get("_SYSTEMD_UNIT").or_else(|| get("_SYSTEMD_USER_UNIT")),
            invocation: get(invocation_key).map(|value| InvocationIdentity {
                field: invocation_key.to_owned(),
                value,
            }),
            job_id: get("JOB_ID"),
            task_id: get("TASK_ID"),
            pid: get("_PID"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    pub machine_id: Option<String>,
    pub boot_id: Option<String>,
    pub unit: Option<String>,
    pub emitter_unit: Option<String>,
    pub invocation: Option<InvocationIdentity>,
    pub job_id: Option<String>,
    pub task_id: Option<String>,
    pub pid: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InvocationIdentity {
    pub field: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Observation {
    First,
    SameInvocation,
    InvocationChanged,
    BootChanged,
    Unknown,
}

pub fn observe(previous: Option<&Identity>, next: &Identity) -> Observation {
    let Some(previous) = previous else {
        return Observation::First;
    };
    if previous.machine_id.is_none() || previous.machine_id != next.machine_id || previous.unit.is_none() || previous.unit != next.unit {
        return Observation::Unknown;
    }
    match (&previous.boot_id, &next.boot_id) {
        (Some(a), Some(b)) if a != b => return Observation::BootChanged,
        (Some(_), Some(_)) => (),
        _ => return Observation::Unknown,
    }
    match (&previous.invocation, &next.invocation) {
        (Some(a), Some(b)) if a.field != b.field => Observation::Unknown,
        (Some(a), Some(b)) if a.value != b.value => Observation::InvocationChanged,
        (Some(_), Some(_)) => Observation::SameInvocation,
        _ => Observation::Unknown,
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub version: u32,
    pub source: Source,
    pub initial_position: InitialPosition,
    pub cursor: Option<String>,
    pub identity: Option<Identity>,
    pub emitted: u64,
}

impl Checkpoint {
    pub fn new(source: Source) -> Result<Self> {
        Self::new_at(source, InitialPosition::Head)
    }
    pub fn new_at(source: Source, initial_position: InitialPosition) -> Result<Self> {
        source.validate()?;
        Ok(Self {
            version: 1,
            source,
            initial_position,
            cursor: None,
            identity: None,
            emitted: 0,
        })
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "unsupported checkpoint version {}", self.version);
        self.source.validate()?;
        ensure!(self.cursor.is_some() == self.identity.is_some(), "cursor and identity must be paired");
        if let Some(cursor) = &self.cursor {
            ensure!(!cursor.is_empty() && !cursor.chars().any(char::is_control), "invalid checkpoint cursor");
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let state: Self = load_json(path)?;
        state.validate()?;
        Ok(state)
    }
    pub fn advance(&mut self, event: &Event) -> Result<Observation> {
        ensure!(self.source.accepts(event), "entry does not match checkpoint source");
        let cursor = event.cursor()?.to_owned();
        let identity = event.identity(&self.source);
        let observation = observe(self.identity.as_ref(), &identity);
        let emitted = self.emitted.checked_add(1).context("emitted counter overflow")?;
        self.cursor = Some(cursor);
        self.identity = Some(identity);
        self.emitted = emitted;
        Ok(observation)
    }
    /// Emit one observation line, flush it, then persist its position. The caller
    /// must hold CheckpointLock. Preflight failure writes nothing; output/flush
    /// failure leaves the saved state unchanged. A save failure can follow emitted
    /// output and, after rename, can leave uncertain persistence. Stop and inspect
    /// disk state before retrying. In-memory state changes only after save success.
    pub fn emit_into(&mut self, event: &Event, output: &mut impl Write, path: &Path) -> Result<Observation> {
        self.validate()?;
        let mut next = self.clone();
        let observation = next.advance(event)?;
        let checkpoint = next.bytes()?;
        let mut line = serde_json::to_vec(&serde_json::json!({
            "observation": observation,
            "identity": next.identity,
            "event": event,
        }))?;
        line.push(b'\n');
        output.write_all(&line).context("output failed; checkpoint unchanged")?;
        output.flush().context("output flush failed; checkpoint unchanged")?;
        Self::persist(&checkpoint, path, false).context("output flushed but checkpoint save failed; inspect state before retrying")?;
        *self = next;
        Ok(observation)
    }
    /// Caller must hold CheckpointLock when coordinating multiple writers.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.write_atomic(path, false)
    }
    /// Atomically create a checkpoint without replacing a path created by a racer.
    pub fn save_new(&self, path: &Path) -> Result<()> {
        self.write_atomic(path, true)
    }
    fn write_atomic(&self, path: &Path, no_clobber: bool) -> Result<()> {
        Self::persist(&self.bytes()?, path, no_clobber)
    }
    fn bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut bytes = serde_json::to_vec_pretty(self)?;
        bytes.push(b'\n');
        ensure!(bytes.len() <= MAX_STATE_BYTES, "checkpoint exceeds 64 KiB");
        Ok(bytes)
    }
    fn persist(bytes: &[u8], path: &Path, no_clobber: bool) -> Result<()> {
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        if no_clobber {
            temporary.persist_noclobber(path)?;
        } else {
            temporary.persist(path)?;
        }
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}

/// Advisory sidecar lock; it is intentionally retained on disk after release.
pub struct CheckpointLock {
    _file: File,
}
impl CheckpointLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        let mut name = path.as_os_str().to_os_string();
        name.push(".lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(PathBuf::from(name))?;
        fs2::FileExt::try_lock_exclusive(&file).context("checkpoint is locked by another writer")?;
        Ok(Self { _file: file })
    }
}

pub fn reference(path: &Path) -> Result<String> {
    let absolute = path.canonicalize()?;
    url::Url::from_file_path(absolute)
        .map(String::from)
        .map_err(|()| anyhow::anyhow!("cannot form file URI"))
}

/// Accept a native path or the local, absolute file URI printed by `reference`.
/// Query/fragment and remote hosts are rejected, never treated as checkpoint data.
/// Non-UTF-8 native paths remain supported; URI percent decoding is delegated to url.
pub fn checkpoint_path(value: &OsStr) -> Result<PathBuf> {
    if !value.as_encoded_bytes().starts_with(b"file:") {
        return Ok(PathBuf::from(value));
    }
    let text = value.to_str().context("file URI must be UTF-8")?;
    ensure!(text.starts_with("file://"), "checkpoint file URI must be absolute (file://)");
    let uri = url::Url::parse(text)?;
    ensure!(uri.query().is_none() && uri.fragment().is_none(), "checkpoint URI query/fragment unsupported");
    uri.to_file_path()
        .map_err(|()| anyhow::anyhow!("checkpoint URI must name a local absolute file"))
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub entries: usize,
    pub line_bytes: usize,
    pub scan_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 100,
            line_bytes: 8 * 1024 * 1024,
            scan_bytes: 64 * 1024 * 1024,
        }
    }
}

// Kill and reap on every early return, including parsing, consumer and budget errors.
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Finite read in input traversal order. Consumer success means the entry was accepted.
/// The journal adapter conservatively compares emitted cursor strings, not native cursor equivalence.
pub fn read(checkpoint: &Checkpoint, limits: Limits, mut consume: impl FnMut(Event) -> Result<()>) -> Result<usize> {
    checkpoint.validate()?;
    ensure!(
        limits.entries > 0 && limits.line_bytes > 0 && limits.scan_bytes > 0 && (limits.line_bytes as u128) < u64::MAX as u128,
        "read limits must be positive and line bound below u64::MAX"
    );
    let fixture_tail =
        matches!(checkpoint.source.backend, Backend::Fixture { .. }) && checkpoint.cursor.is_none() && checkpoint.initial_position == InitialPosition::Tail;
    let mut tail_entries = VecDeque::new();
    let mut process = None;
    let reader: Box<dyn Read> = match &checkpoint.source.backend {
        Backend::Fixture { path } => Box::new(File::open(path)?),
        Backend::Journal { scope, directory } => {
            let mut command = Command::new("journalctl");
            command.args(["--no-pager", "--quiet", "--output=json", "--all"]);
            command.arg(match scope {
                Scope::System => "--system",
                Scope::User => "--user",
            });
            if let Some(directory) = directory {
                command.arg("--directory").arg(directory);
            }
            if let Some(cursor) = &checkpoint.cursor {
                command.arg(format!("--cursor={cursor}"));
                command.arg("--lines=all");
            } else {
                match checkpoint.initial_position {
                    InitialPosition::Head => {
                        command.arg("--lines=all");
                    }
                    InitialPosition::Tail => {
                        command.arg(format!("--lines={}", limits.entries));
                    }
                }
            }
            command.arg("--");
            for (key, value) in checkpoint.source.selectors() {
                command.arg(format!("{key}={value}"));
            }
            let mut child = Process(command.stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().context("spawn journalctl")?);
            let stdout = child.0.stdout.take().context("journalctl stdout missing")?;
            process = Some(child);
            Box::new(stdout)
        }
    };
    let strict_first = process.is_some();
    let mut reader = BufReader::new(reader);
    let mut found = checkpoint.cursor.is_none();
    let mut count = 0;
    let mut scanned = 0usize;
    loop {
        let mut bytes = Vec::new();
        let size = reader.by_ref().take(limits.line_bytes as u64 + 1).read_until(b'\n', &mut bytes)?;
        if size == 0 {
            break;
        }
        ensure!(size <= limits.line_bytes, "journal line exceeds byte budget");
        scanned = scanned.checked_add(size).context("scan counter overflow")?;
        ensure!(scanned <= limits.scan_bytes, "journal scan exceeds byte budget");
        let event: Event = serde_json::from_slice(&bytes).context("invalid journal JSON entry")?;
        let cursor = event.cursor()?;
        if fixture_tail {
            if checkpoint.source.accepts(&event) {
                tail_entries.push_back(event);
                if tail_entries.len() > limits.entries {
                    tail_entries.pop_front();
                }
            }
            continue;
        }
        if !found {
            if Some(cursor) == checkpoint.cursor.as_deref() {
                ensure!(checkpoint.source.accepts(&event), "saved cursor entry no longer matches source");
                found = true;
            } else if strict_first {
                bail!("cursor unavailable or differently spelled; refusing nearest-entry resume");
            }
            continue;
        }
        if !checkpoint.source.accepts(&event) {
            ensure!(!strict_first, "journalctl returned entry outside source selection");
            continue;
        }
        consume(event)?;
        count += 1;
        if count == limits.entries {
            return Ok(count);
        }
    }
    if let Some(process) = &mut process {
        ensure!(process.0.wait()?.success(), "journalctl failed");
    }
    ensure!(found, "saved cursor unavailable; refusing reset");
    if fixture_tail {
        for event in tail_entries {
            consume(event)?;
            count += 1;
        }
    }
    Ok(count)
}
