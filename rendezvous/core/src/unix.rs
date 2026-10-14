use std::{
    error::Error,
    ffi::{OsStr, OsString},
    fmt, io,
    os::unix::net::SocketAddr,
    path::{Path, PathBuf},
};

/// Per-name limit for service names and namespaces, in UTF-8 bytes, sized to
/// the usual single filesystem component limit. It bounds each name alone:
/// a filename combining a namespace, a service name, and separators can
/// still exceed a particular filesystem's component limit. That limit is
/// checked by filesystem operations, not name validation. Separately, the
/// full socket address must fit the platform's Unix socket address capacity,
/// checked by [`Endpoint::socket_addr`].
pub const MAX_NAME_BYTES: usize = 255;

/// A caller-owned rendezvous name, validated once so resolvers can trust it.
///
/// Nonempty, at most [`MAX_NAME_BYTES`] bytes, no `/` separator, no ASCII
/// control characters (NUL included). Names are UTF-8; Unix path bytes that
/// are not UTF-8 belong in [`Endpoint`] paths, not in names. Because the
/// separator is rejected, a validated name embedded between a prefix and a
/// suffix can never form `.`/`..` or escape its base directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceName(String);

/// Why a service name or namespace was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidName {
    Empty,
    Separator,
    Control,
    TooLong,
}
impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "service name must not be empty",
            Self::Separator => "service name must not contain '/'",
            Self::Control => "service name must not contain control characters",
            Self::TooLong => "service name exceeds the maximum byte length",
        })
    }
}
impl Error for InvalidName {}

impl ServiceName {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidName> {
        let value = value.into();
        if value.is_empty() {
            Err(InvalidName::Empty)
        } else if value.contains('/') {
            Err(InvalidName::Separator)
        } else if value.bytes().any(|byte| byte < 0x20 || byte == 0x7f) {
            Err(InvalidName::Control)
        } else if value.len() > MAX_NAME_BYTES {
            Err(InvalidName::TooLong)
        } else {
            Ok(Self(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for ServiceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A Unix-domain endpoint address: a filesystem path or a Linux abstract name.
///
/// This is address data only. It performs and implies no I/O; use
/// [`Endpoint::socket_addr`] when about to connect. The Linux abstract
/// namespace is included because the targeted manager ecosystem names
/// abstract sockets directly (`@name` in systemd socket units) and abstract
/// endpoints leave no stale socket file for this toolkit's deliberate
/// no-deletion stance to worry about.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Endpoint {
    /// Filesystem socket path. Native bytes are preserved; non-UTF-8 is fine.
    Path(PathBuf),
    /// Abstract namespace name, stored without the address-form leading NUL.
    Abstract(AbstractName),
}

/// A nonempty abstract-namespace name without interior NUL bytes.
///
/// Deliberately stricter than the kernel: Linux length-delimits abstract
/// names and would accept interior NULs and the empty name. Both are
/// rejected here so every `AbstractName` also round-trips through the
/// NUL-free text convention of [`Endpoint::parse_value`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbstractName(Vec<u8>);
impl AbstractName {
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Result<Self, InvalidEndpoint> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            Err(InvalidEndpoint::Empty)
        } else if bytes.contains(&0) {
            Err(InvalidEndpoint::InteriorNul)
        } else {
            Ok(Self(bytes))
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Display for AbstractName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", String::from_utf8_lossy(&self.0))
    }
}

/// Why endpoint data was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidEndpoint {
    Empty,
    InteriorNul,
}
impl fmt::Display for InvalidEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "endpoint must not be empty",
            Self::InteriorNul => "endpoint must not contain interior NUL bytes",
        })
    }
}
impl Error for InvalidEndpoint {}

impl Endpoint {
    /// Any nonempty path without interior NUL bytes; canonicalization is not
    /// performed and relative paths stay relative.
    pub fn path(path: impl Into<PathBuf>) -> Result<Self, InvalidEndpoint> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            Err(InvalidEndpoint::Empty)
        } else if path.as_os_str().as_encoded_bytes().contains(&0) {
            Err(InvalidEndpoint::InteriorNul)
        } else {
            Ok(Self::Path(path))
        }
    }

    /// Validated abstract name; see [`AbstractName`].
    pub fn abstract_name(name: impl Into<Vec<u8>>) -> Result<Self, InvalidEndpoint> {
        Ok(Self::Abstract(AbstractName::from_bytes(name)?))
    }

    /// Parses the shared text convention used by environment values and CLI
    /// arguments: a leading `@` selects the abstract namespace (systemd
    /// socket-unit style); anything else is a filesystem path, preserved
    /// byte-for-byte including non-UTF-8. A path that genuinely starts with
    /// `@` has no text form; construct it with [`Endpoint::path`] instead.
    pub fn parse_value(value: &OsStr) -> Result<Self, InvalidEndpoint> {
        let bytes = value.as_encoded_bytes();
        if let Some(rest) = bytes.strip_prefix(b"@") {
            Self::abstract_name(rest)
        } else {
            Self::path(value)
        }
    }

    pub fn as_path(&self) -> Option<&Path> {
        match self {
            Self::Path(path) => Some(path),
            Self::Abstract(_) => None,
        }
    }

    pub fn as_abstract(&self) -> Option<&AbstractName> {
        match self {
            Self::Path(_) => None,
            Self::Abstract(name) => Some(name),
        }
    }

    /// Address conversion for a caller about to connect. Performs no I/O.
    /// Both address kinds are capacity-bounded: a filesystem path longer
    /// than the kernel address structure and an abstract name exceeding the
    /// abstract-namespace limit both error here rather than silently
    /// truncating. Abstract names need Linux
    /// (`std::os::linux::net::SocketAddrExt`) and return
    /// [`io::ErrorKind::Unsupported`] elsewhere.
    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        match self {
            Self::Path(path) => SocketAddr::from_pathname(path),
            #[cfg(target_os = "linux")]
            Self::Abstract(name) => {
                <SocketAddr as std::os::linux::net::SocketAddrExt>::from_abstract_name(
                    name.as_bytes(),
                )
            }
            #[cfg(not(target_os = "linux"))]
            Self::Abstract(_) => Err(io::Error::from(io::ErrorKind::Unsupported)),
        }
    }
}
impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Path(path) => write!(f, "{}", path.display()),
            Self::Abstract(name) => write!(f, "{name}"),
        }
    }
}

/// Where a candidate came from, for diagnostics only; a free-form label a
/// policy assigns (for example `explicit`, `environment:SOCKET_VAR`,
/// `xdg-runtime-dir`). Labels are provenance hints, not configuration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Origin(String);
impl Origin {
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One unverified address a resolver proposes, with its source label.
///
/// A candidate asserts configuration, never reachability: the endpoint may
/// not exist, may be a stale file with no listener behind it, and nothing
/// about it has been authenticated. Attempting a connection, interpreting
/// the failure, and retrying are caller-owned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub endpoint: Endpoint,
    pub origin: Origin,
}
impl Candidate {
    pub fn new(endpoint: Endpoint, origin: Origin) -> Self {
        Self { endpoint, origin }
    }
}

/// Resolution failure. Invalid *input data* fails predictably; an absent
/// configuration is not an error and yields no candidates instead.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolveError {
    Name(InvalidName),
    /// A consulted source supplied endpoint data that cannot be used. This is
    /// the predictable-fail rule: it is never silently skipped, and later
    /// sources are not consulted as a fallback.
    Endpoint {
        origin: Origin,
        invalid: InvalidEndpoint,
    },
}
impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(invalid) => write!(f, "invalid service name: {invalid}"),
            Self::Endpoint { origin, invalid } => {
                write!(f, "invalid endpoint from {origin}: {invalid}")
            }
        }
    }
}
impl Error for ResolveError {}

/// The rendezvous contract: ordered, unverified candidates for a name.
///
/// Implementations derive data; they never connect, start a server,
/// authenticate a peer, or remove socket files. An empty result means "no
/// configured source contributed anything" and is not an error — deciding
/// what to do then (including any opt-in short-lived server) is caller
/// policy that deliberately lives outside this trait. The trait is
/// object-safe so consumers can accept `&dyn Resolver`.
///
/// ```
/// use handle_off_rendezvous::{
///     Candidate, Endpoint, Origin, ResolveError, Resolver, ServiceName,
/// };
///
/// struct Fixed(Endpoint);
/// impl Resolver for Fixed {
///     fn candidates(&self, name: &ServiceName) -> Result<Vec<Candidate>, ResolveError> {
///         assert_eq!(name.as_str(), "ctl");
///         Ok(vec![Candidate::new(
///             self.0.clone(),
///             Origin::new("fixed"),
///         )])
///     }
/// }
///
/// let name = ServiceName::new("ctl").unwrap();
/// let found = Fixed(Endpoint::abstract_name(b"demo").unwrap())
///     .candidates(&name)
///     .unwrap();
/// assert_eq!(found[0].endpoint.as_abstract().unwrap().as_bytes(), b"demo");
/// assert_eq!(found[0].origin.as_str(), "fixed");
/// ```
pub trait Resolver {
    fn candidates(&self, name: &ServiceName) -> Result<Vec<Candidate>, ResolveError>;
}

/// A read-only environment snapshot used as resolution input.
///
/// Policies never read or mutate the live process environment during
/// resolution; they consult a snapshot like this one. [`Environment::capture`]
/// is a convenience that copies the current environment once (not atomically;
/// it is not a coordination mechanism — filter a child's environment at
/// spawn time instead). Duplicate keys are preserved as captured; lookups
/// return the first match. Values and keys keep their native bytes,
/// including non-UTF-8.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    entries: Vec<(OsString, OsString)>,
}
impl Environment {
    pub fn capture() -> Self {
        Self {
            entries: std::env::vars_os().collect(),
        }
    }
    pub fn from_pairs(pairs: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        Self {
            entries: pairs.into_iter().collect(),
        }
    }
    /// First match by exact native-byte key.
    pub fn get(&self, name: impl AsRef<OsStr>) -> Option<&OsStr> {
        let name = name.as_ref();
        self.entries
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_os_str())
    }
    /// Snapshot order, as captured or constructed.
    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    fn name(value: impl Into<String>) -> Result<ServiceName, InvalidName> {
        ServiceName::new(value)
    }

    #[test]
    fn service_name_rules() {
        assert!(name("ctl").is_ok());
        assert!(name("café").is_ok());
        assert!(name("a".repeat(MAX_NAME_BYTES)).is_ok());
        assert_eq!(name(""), Err(InvalidName::Empty));
        assert_eq!(name("a/b"), Err(InvalidName::Separator));
        assert_eq!(name("a\nb"), Err(InvalidName::Control));
        assert_eq!(name("a\0b"), Err(InvalidName::Control));
        assert_eq!(name("a\u{7f}"), Err(InvalidName::Control));
        assert_eq!(
            name("a".repeat(MAX_NAME_BYTES + 1)),
            Err(InvalidName::TooLong)
        );
    }

    #[test]
    fn path_endpoint_rules() {
        assert_eq!(
            Endpoint::path("/run/demo.sock").unwrap(),
            Endpoint::Path(PathBuf::from("/run/demo.sock"))
        );
        assert_eq!(Endpoint::path(""), Err(InvalidEndpoint::Empty));
        assert_eq!(
            Endpoint::path(PathBuf::from(OsString::from_vec(b"a\0b".to_vec()))),
            Err(InvalidEndpoint::InteriorNul)
        );
    }

    #[test]
    fn abstract_endpoint_rules() {
        let endpoint = Endpoint::abstract_name(b"demo").unwrap();
        assert_eq!(endpoint.as_abstract().unwrap().as_bytes(), b"demo");
        assert!(endpoint.as_path().is_none());
        assert_eq!(Endpoint::abstract_name(b""), Err(InvalidEndpoint::Empty));
        assert_eq!(
            Endpoint::abstract_name(b"a\0b"),
            Err(InvalidEndpoint::InteriorNul)
        );
    }

    #[test]
    fn value_parsing_keeps_the_systemd_at_convention_and_native_bytes() {
        assert_eq!(
            Endpoint::parse_value(OsStr::new("/run/demo.sock")).unwrap(),
            Endpoint::path("/run/demo.sock").unwrap()
        );
        assert_eq!(
            Endpoint::parse_value(OsStr::new("@demo")).unwrap(),
            Endpoint::abstract_name(b"demo").unwrap()
        );
        assert_eq!(
            Endpoint::parse_value(OsStr::new("@")),
            Err(InvalidEndpoint::Empty)
        );
        assert_eq!(
            Endpoint::parse_value(OsStr::new("")),
            Err(InvalidEndpoint::Empty)
        );
        let non_utf8_path = OsString::from_vec(b"/tmp/demo-\xff.sock".to_vec());
        let parsed = Endpoint::parse_value(&non_utf8_path).unwrap();
        assert_eq!(
            parsed.as_path().unwrap().as_os_str().as_bytes(),
            b"/tmp/demo-\xff.sock"
        );
        let non_utf8_abstract = OsString::from_vec(b"@\xff\xfe".to_vec());
        let parsed = Endpoint::parse_value(&non_utf8_abstract).unwrap();
        assert_eq!(parsed.as_abstract().unwrap().as_bytes(), b"\xff\xfe");
    }

    #[test]
    fn display_is_lossy_diagnostics_only() {
        // Lossy rendering only: non-UTF-8 bytes become replacement
        // characters, and distinct byte sequences can render identically.
        // Display output is for humans and logs, never for matching.
        assert_eq!(
            Endpoint::path("/run/demo.sock").unwrap().to_string(),
            "/run/demo.sock"
        );
        assert_eq!(
            Endpoint::abstract_name("demø".as_bytes().to_vec())
                .unwrap()
                .to_string(),
            "@demø"
        );
    }

    #[test]
    fn socket_addr_conversion_round_trips_and_reports_limits() {
        let path_addr = Endpoint::path("/run/demo.sock")
            .unwrap()
            .socket_addr()
            .unwrap();
        assert_eq!(path_addr.as_pathname(), Some(Path::new("/run/demo.sock")));
        let abstract_addr = Endpoint::abstract_name(b"demo").unwrap().socket_addr();
        #[cfg(target_os = "linux")]
        assert_eq!(
            std::os::linux::net::SocketAddrExt::as_abstract_name(&abstract_addr.unwrap()),
            Some(b"demo".as_slice())
        );
        #[cfg(not(target_os = "linux"))]
        assert_eq!(
            abstract_addr.unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        #[cfg(target_os = "linux")]
        assert!(
            Endpoint::abstract_name(vec![b'a'; 200])
                .unwrap()
                .socket_addr()
                .is_err()
        );
        let long = Endpoint::path(format!("/tmp/{}", "a".repeat(200)))
            .unwrap()
            .socket_addr();
        assert!(long.is_err());
    }

    #[test]
    fn environment_snapshot_lookup_and_order() {
        let non_utf8_key = OsString::from_vec(b"KEY_\xff".to_vec());
        let non_utf8_value = OsString::from_vec(b"/tmp/\xff.sock".to_vec());
        let snapshot = Environment::from_pairs([
            ("FIRST".into(), "one".into()),
            (non_utf8_key.clone(), non_utf8_value.clone()),
            ("FIRST".into(), "two".into()),
        ]);
        assert_eq!(snapshot.get("FIRST"), Some(OsStr::new("one")));
        assert_eq!(
            snapshot.get(&non_utf8_key).map(|value| value.as_bytes()),
            Some(non_utf8_value.as_bytes())
        );
        assert_eq!(snapshot.get("ABSENT"), None);
        assert_eq!(
            snapshot
                .iter()
                .map(|(key, _)| key.to_string_lossy())
                .collect::<Vec<_>>(),
            vec!["FIRST", "KEY_\u{fffd}", "FIRST"],
        );
    }

    #[test]
    fn capture_reads_without_mutating() {
        let before = std::env::vars_os().collect::<Vec<_>>();
        let snapshot = Environment::capture();
        let after = std::env::vars_os().collect::<Vec<_>>();
        assert_eq!(snapshot.iter().count(), before.len());
        assert_eq!(before.len(), after.len());
    }

    #[test]
    fn resolve_error_display_names_the_source() {
        let error = ResolveError::Endpoint {
            origin: Origin::new("environment:DEMO_SOCKET"),
            invalid: InvalidEndpoint::Empty,
        };
        assert_eq!(
            error.to_string(),
            "invalid endpoint from environment:DEMO_SOCKET: endpoint must not be empty"
        );
        assert_eq!(
            ResolveError::Name(InvalidName::Separator).to_string(),
            "invalid service name: service name must not contain '/'"
        );
    }
}
