use handle_off_systemd_fdstore::{FdName, ManagerReport};
use std::{
    collections::{BTreeSet, HashMap},
    error::Error as StdError,
    fmt, io,
    os::fd::BorrowedFd,
};
pub const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 65_536;
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json(serde_json::Error),
    Xml(roxmltree::Error),
    InvalidConfiguration(&'static str),
    InvalidReply(&'static str),
    Timeout,
    OutputLimit,
    PermissionDenied(String),
    Unsupported(&'static str),
    StoreDisabled,
    CommandFailed { status: Option<i32>, stderr: String },
    WrongUnit { expected: String, actual: String },
    OwnerMismatch { expected: u32, actual: u32 },
    IdentityChanged,
    InconsistentInventory,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "systemd inspection error: {self:?}")
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::Xml(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<rustix::io::Errno> for Error {
    fn from(error: rustix::io::Errno) -> Self {
        Self::Io(error.into())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelIdentity {
    pub mode: u32,
    pub device_major: u32,
    pub device_minor: u32,
    pub inode: u64,
    pub rdev_major: u32,
    pub rdev_minor: u32,
}
impl KernelIdentity {
    pub fn from_fd(fd: BorrowedFd<'_>) -> io::Result<Self> {
        let stat = rustix::fs::fstat(fd)?;
        Ok(Self {
            mode: stat.st_mode,
            device_major: rustix::fs::major(stat.st_dev),
            device_minor: rustix::fs::minor(stat.st_dev),
            inode: stat.st_ino,
            rdev_major: rustix::fs::major(stat.st_rdev),
            rdev_minor: rustix::fs::minor(stat.st_rdev),
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredEntry {
    pub name: FdName,
    pub identity: KernelIdentity,
    pub path: String,
    pub flags: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreContents {
    Available(Vec<StoredEntry>),
    Unsupported,
    Disabled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceIdentity {
    pub unit: String,
    pub object_path: String,
    pub main_pid: u32,
    pub invocation_id: [u8; 16],
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub identity: ServiceIdentity,
    pub manager_version: String,
    pub report: ManagerReport,
    pub contents: StoreContents,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Membership {
    MatchedMetadata { unique_identities: usize },
    Missing,
    Unexpected,
    Ambiguous,
    Unsupported,
    Disabled,
}
impl Snapshot {
    /// Observed membership only. Expected duplicate metadata classes collapse;
    /// aliases/OFD sharing cannot be inferred. Duplicate observed rows with
    /// indistinguishable identity cannot be certified and return Ambiguous.
    pub fn membership(&self, name: &FdName, originals: &[KernelIdentity]) -> Membership {
        let entries = match &self.contents {
            StoreContents::Available(entries) => entries,
            StoreContents::Unsupported => return Membership::Unsupported,
            StoreContents::Disabled => return Membership::Disabled,
        };
        if originals.is_empty() {
            return Membership::Missing;
        }
        let expected: BTreeSet<_> = originals.iter().collect();
        let mut found = BTreeSet::new();
        for entry in entries.iter().filter(|entry| &entry.name == name) {
            if !expected.contains(&entry.identity) {
                return Membership::Unexpected;
            }
            if !found.insert(&entry.identity) {
                return Membership::Ambiguous;
            }
        }
        if found != expected {
            return Membership::Missing;
        }
        Membership::MatchedMetadata {
            unique_identities: expected.len(),
        }
    }
}
pub(crate) fn reply(bytes: &[u8], signature: &str) -> Result<serde_json::Value, Error> {
    if bytes.len() > MAX_OUTPUT_BYTES {
        return Err(Error::OutputLimit);
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(Error::Json)?;
    if value.get("type").and_then(|v| v.as_str()) != Some(signature) {
        return Err(Error::InvalidReply("unexpected D-Bus signature"));
    }
    value
        .get("data")
        .cloned()
        .ok_or(Error::InvalidReply("missing typed data"))
}
pub(crate) fn uint32(value: &serde_json::Value) -> Result<u32, Error> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(Error::InvalidReply("expected uint32"))
}
pub(crate) fn string(value: &serde_json::Value) -> Result<String, Error> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or(Error::InvalidReply("expected string"))
}
pub(crate) fn single(value: &serde_json::Value) -> Result<&serde_json::Value, Error> {
    let list = value
        .as_array()
        .ok_or(Error::InvalidReply("expected one call result"))?;
    if list.len() != 1 {
        return Err(Error::InvalidReply("expected exactly one call result"));
    }
    Ok(&list[0])
}
/// Parse busctl's typed call output, preserving all nonunique names. Pure parsing;
/// no acceptance is inferred from JSON validity or from entry counts.
pub fn parse_dump(bytes: &[u8], max_entries: usize) -> Result<Vec<StoredEntry>, Error> {
    if max_entries > MAX_ENTRIES {
        return Err(Error::InvalidConfiguration("entry limit exceeds ceiling"));
    }
    let data = reply(bytes, "a(suuutuusu)")?;
    let rows = single(&data)?
        .as_array()
        .ok_or(Error::InvalidReply("expected fdstore row array"))?;
    if rows.len() > max_entries {
        return Err(Error::OutputLimit);
    }
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let row = row
            .as_array()
            .ok_or(Error::InvalidReply("expected fdstore tuple"))?;
        if row.len() != 9 {
            return Err(Error::InvalidReply("fdstore tuple must have nine fields"));
        }
        let name = FdName::new(string(&row[0])?)
            .map_err(|_| Error::InvalidReply("invalid manager FDNAME"))?;
        entries.push(StoredEntry {
            name,
            identity: KernelIdentity {
                mode: uint32(&row[1])?,
                device_major: uint32(&row[2])?,
                device_minor: uint32(&row[3])?,
                inode: row[4]
                    .as_u64()
                    .ok_or(Error::InvalidReply("expected uint64 inode"))?,
                rdev_major: uint32(&row[5])?,
                rdev_minor: uint32(&row[6])?,
            },
            path: string(&row[7])?,
            flags: uint32(&row[8])?,
        });
    }
    Ok(entries)
}
pub(crate) struct Schema {
    properties: HashMap<(String, String), String>,
    pub dump: bool,
}
impl Schema {
    pub fn parse(xml: &str) -> Result<Self, Error> {
        if xml.len() > MAX_OUTPUT_BYTES {
            return Err(Error::OutputLimit);
        }
        // systemd includes the standard external introspection DOCTYPE. Do not
        // resolve it over the network, and reject internal entity declarations.
        if xml.contains("<!ENTITY") {
            return Err(Error::InvalidReply(
                "introspection entity declarations unsupported",
            ));
        }
        let options = roxmltree::ParsingOptions {
            allow_dtd: true,
            nodes_limit: 32_768,
            entity_resolver: None,
        };
        let document = roxmltree::Document::parse_with_options(xml, options).map_err(Error::Xml)?;
        if document.root_element().tag_name().name() != "node" {
            return Err(Error::InvalidReply("invalid introspection root"));
        }
        let mut properties = HashMap::new();
        let mut dump = false;
        for interface in document
            .root_element()
            .children()
            .filter(|node| node.has_tag_name("interface"))
        {
            let Some(interface_name) = interface.attribute("name") else {
                return Err(Error::InvalidReply("unnamed interface"));
            };
            for member in interface.children().filter(|node| node.is_element()) {
                if member.has_tag_name("property") {
                    let name = member
                        .attribute("name")
                        .ok_or(Error::InvalidReply("unnamed property"))?;
                    let kind = member
                        .attribute("type")
                        .ok_or(Error::InvalidReply("untyped property"))?;
                    if properties
                        .insert((interface_name.into(), name.into()), kind.into())
                        .is_some()
                    {
                        return Err(Error::InvalidReply("duplicate introspection property"));
                    }
                }
                if interface_name == "org.freedesktop.systemd1.Service"
                    && member.has_tag_name("method")
                    && member.attribute("name") == Some("DumpFileDescriptorStore")
                {
                    let arguments: Vec<_> = member
                        .children()
                        .filter(|node| node.has_tag_name("arg"))
                        .collect();
                    if dump
                        || arguments.len() != 1
                        || arguments[0].attribute("direction") != Some("out")
                        || arguments[0].attribute("type") != Some("a(suuutuusu)")
                    {
                        return Err(Error::InvalidReply("unsupported dump method signature"));
                    }
                    dump = true;
                }
            }
        }
        Ok(Self { properties, dump })
    }
    pub fn property(&self, interface: &str, name: &str, signature: &str) -> Result<bool, Error> {
        match self.properties.get(&(interface.into(), name.into())) {
            None => Ok(false),
            Some(kind) if kind == signature => Ok(true),
            Some(_) => Err(Error::InvalidReply("unexpected introspected property type")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn row(name: &str, inode: u64) -> serde_json::Value {
        json!([name, 49152, 0, 9, inode, 0, 0, "socket", 2])
    }
    fn snapshot(rows: Vec<serde_json::Value>) -> Snapshot {
        let bytes = serde_json::to_vec(&json!({"type":"a(suuutuusu)", "data":[rows]})).unwrap();
        Snapshot {
            identity: ServiceIdentity {
                unit: "demo.service".into(),
                object_path: "/org/freedesktop/systemd1/unit/demo_2eservice".into(),
                main_pid: 42,
                invocation_id: [1; 16],
            },
            manager_version: "262".into(),
            report: ManagerReport::default(),
            contents: StoreContents::Available(parse_dump(&bytes, 16).unwrap()),
        }
    }
    fn identity(inode: u64) -> KernelIdentity {
        KernelIdentity {
            mode: 49152,
            device_major: 0,
            device_minor: 9,
            inode,
            rdev_major: 0,
            rdev_minor: 0,
        }
    }
    #[test]
    fn all_named_rows_must_match_and_no_hashmap_overwrites_duplicates() {
        let name = FdName::new("shared").unwrap();
        assert_eq!(
            snapshot(vec![row("shared", 1), row("shared", 2)])
                .membership(&name, &[identity(1), identity(2)]),
            Membership::MatchedMetadata {
                unique_identities: 2
            }
        );
        assert_eq!(
            snapshot(vec![row("shared", 1), row("shared", 2)]).membership(&name, &[identity(1)]),
            Membership::Unexpected
        );
        assert_eq!(
            snapshot(vec![row("shared", 1), row("shared", 1)]).membership(&name, &[identity(1)]),
            Membership::Ambiguous
        );
        assert_eq!(
            snapshot(vec![row("other", 1)]).membership(&name, &[identity(1)]),
            Membership::Missing
        );
        assert_eq!(
            snapshot(vec![]).membership(&name, &[identity(1)]),
            Membership::Missing
        );
        assert_eq!(
            snapshot(vec![row("shared", 1)]).membership(&name, &[identity(1), identity(1)]),
            Membership::MatchedMetadata {
                unique_identities: 1
            }
        );
    }
    #[test]
    fn dump_json_is_bounded_typed_and_structurally_checked() {
        for bytes in [
            b"{}".as_slice(),
            b"garbage",
            br#"{"type":"a(suuutuusu)","data":[[["x",1]]]}"#,
            br#"{"type":"u","data":1}"#,
        ] {
            assert!(parse_dump(bytes, 16).is_err());
        }
        let mut bad = row("x", 1);
        bad[1] = json!(u64::MAX);
        let bytes = serde_json::to_vec(&json!({"type":"a(suuutuusu)","data":[[bad]]})).unwrap();
        assert!(parse_dump(&bytes, 16).is_err());
        let bytes =
            serde_json::to_vec(&json!({"type":"a(suuutuusu)","data":[[row("x",u64::MAX)]]}))
                .unwrap();
        assert_eq!(parse_dump(&bytes, 1).unwrap()[0].identity.inode, u64::MAX);
        assert!(matches!(parse_dump(&bytes, 0), Err(Error::OutputLimit)));
        assert!(parse_dump(&bytes, MAX_ENTRIES + 1).is_err());
        for length in 0..bytes.len() {
            assert!(parse_dump(&bytes[..length], 1).is_err());
        }
    }
    #[test]
    fn introspection_accepts_standard_external_doctype_without_entity_resolution() {
        let xml = r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN" "https://www.freedesktop.org/standards/dbus/1.0/introspect.dtd"><node><interface name="org.freedesktop.systemd1.Service"><property name="MainPID" type="u" access="read"/><method name="DumpFileDescriptorStore"><arg direction="out" type="a(suuutuusu)"/></method></interface></node>"#;
        let schema = Schema::parse(xml).unwrap();
        assert!(schema.dump);
        assert!(
            schema
                .property("org.freedesktop.systemd1.Service", "MainPID", "u")
                .unwrap()
        );
        assert!(
            !schema
                .property("org.freedesktop.systemd1.Service", "Absent", "u")
                .unwrap()
        );
        assert!(
            schema
                .property("org.freedesktop.systemd1.Service", "MainPID", "s")
                .is_err()
        );
        assert!(Schema::parse(r#"<!DOCTYPE node [<!ENTITY attack "expanded">]><node/>"#).is_err());
        assert!(Schema::parse("<invalid/>").is_err());
    }
    #[test]
    fn inode_metadata_does_not_prove_shared_open_file_description() {
        use std::{
            io::{Seek, SeekFrom, Write},
            os::fd::AsFd,
        };
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.test-agent/systemd-inspect");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(format!("ofd-limit-{}", std::process::id()));
        let mut first = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut second = std::fs::File::open(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        first.write_all(b"bytes").unwrap();
        first.seek(SeekFrom::Start(3)).unwrap();
        assert_eq!(
            KernelIdentity::from_fd(first.as_fd()).unwrap(),
            KernelIdentity::from_fd(second.as_fd()).unwrap()
        );
        assert_eq!(first.stream_position().unwrap(), 3);
        assert_eq!(second.stream_position().unwrap(), 0);
    }
}
