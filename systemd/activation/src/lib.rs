//! Classic PID-scoped socket activation, independent of fdstore and bundle schemas.
//! Parse an explicit snapshot first; claiming inherited raw descriptors is a
//! separate unsafe operation. Environment strings and fcntl cannot establish sole
//! ownership. Names are nonunique manager tags, not persistent resource identities.
//! No environment mutation, listener binding, re-storage, or automatic cleanup of
//! manager state occurs. This implements LISTEN_PID/FDS/FDNAMES, not the optional
//! newer LISTEN_PIDFDID process-identity check.
#![deny(unsafe_op_in_unsafe_fn)]
use std::{error::Error, ffi::OsString, fmt};

pub const FIRST_FD: i32 = 3;
pub const MAX_FDS: usize = 65_536;
pub const MAX_NAME_BYTES: usize = 255;

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub listen_pid: Option<OsString>,
    pub listen_fds: Option<OsString>,
    pub listen_fdnames: Option<OsString>,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_fds: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self { max_fds: 4096 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    InvalidLimits,
    MissingPid,
    InvalidPid,
    MissingCount,
    InvalidCount,
    CountOverflow,
    DescriptorRangeOverflow,
    TooManyDescriptors,
    InvalidNames,
    NameCountMismatch,
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "activation metadata error: {self:?}")
    }
}
impl Error for ParseError {}

/// A process-local inherited slot. Index/number describe delivery placement;
/// neither is stable application identity. Missing names remain None rather than
/// conflating absent metadata with a literal manager tag such as "unknown".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescriptorInfo {
    pub index: usize,
    pub number: i32,
    pub name: Option<String>,
}
#[derive(Debug)]
pub struct Metadata {
    descriptors: Vec<DescriptorInfo>,
}
impl Metadata {
    pub fn descriptors(&self) -> &[DescriptorInfo] {
        &self.descriptors
    }
}

impl Snapshot {
    /// Reads only; no environment removal and no descriptor ownership. Callers
    /// needing a coherent snapshot should capture at coordinated early startup.
    /// Later parsing/claiming is not made safe by this convenience method. Repeated
    /// capture does not consume metadata. Child environment filtering or explicitly
    /// coordinated edition-2024 unsafe removal remains the application owner's job.
    pub fn capture() -> Self {
        Self {
            listen_pid: std::env::var_os("LISTEN_PID"),
            listen_fds: std::env::var_os("LISTEN_FDS"),
            listen_fdnames: std::env::var_os("LISTEN_FDNAMES"),
        }
    }
    /// Pure parse. Missing PID AND count means no activation; a foreign valid PID
    /// is ignored before inspecting its count/names. Partial current-process
    /// metadata is an error. Zero count is no activation (names must fit zero).
    /// Names, when present, must have exactly one colon-separated tag per slot;
    /// each tag is ASCII without controls, at most 255 bytes. Segment collection
    /// is bounded by the declared count before the count check, so an over-long
    /// colon run is rejected without materializing one entry per separator.
    /// Empty/repeated tags
    /// are retained, and reserved tags such as "stored" receive no special identity.
    pub fn parse(&self, current_pid: u32, limits: Limits) -> Result<Option<Metadata>, ParseError> {
        if limits.max_fds > MAX_FDS {
            return Err(ParseError::InvalidLimits);
        }
        if self.listen_pid.is_none() && self.listen_fds.is_none() {
            return Ok(None);
        }
        let pid = self
            .listen_pid
            .as_ref()
            .ok_or(ParseError::MissingPid)?
            .to_str()
            .ok_or(ParseError::InvalidPid)?;
        if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseError::InvalidPid);
        }
        let pid = pid.parse::<u32>().map_err(|_| ParseError::InvalidPid)?;
        if pid == 0 {
            return Err(ParseError::InvalidPid);
        }
        if pid != current_pid {
            return Ok(None);
        }
        let count = self
            .listen_fds
            .as_ref()
            .ok_or(ParseError::MissingCount)?
            .to_str()
            .ok_or(ParseError::InvalidCount)?;
        if count.is_empty() || !count.bytes().all(|b| b.is_ascii_digit()) {
            return Err(ParseError::InvalidCount);
        }
        let count = count
            .parse::<u64>()
            .map_err(|_| ParseError::CountOverflow)?;
        if count > (i32::MAX as u64 - FIRST_FD as u64 + 1) {
            return Err(ParseError::DescriptorRangeOverflow);
        }
        if count > limits.max_fds as u64 {
            return Err(ParseError::TooManyDescriptors);
        }
        let count = count as usize;
        let names = if let Some(names) = &self.listen_fdnames {
            let names = names.to_str().ok_or(ParseError::InvalidNames)?;
            if names.len() > count.saturating_mul(MAX_NAME_BYTES + 1) {
                return Err(ParseError::InvalidNames);
            }
            let names: Vec<_> = if count == 0 && names.is_empty() {
                Vec::new()
            } else {
                // Bound collection before the count check: the total-length
                // pre-check alone permits up to count * 256 bytes, and a
                // colon-only value would otherwise materialize a segment per
                // separator before NameCountMismatch rejects it.
                names.split(':').take(count + 1).collect()
            };
            if names.len() != count {
                return Err(ParseError::NameCountMismatch);
            }
            if names.iter().any(|name| {
                name.len() > MAX_NAME_BYTES
                    || !name.is_ascii()
                    || name.bytes().any(|b| b.is_ascii_control())
            }) {
                return Err(ParseError::InvalidNames);
            }
            Some(names)
        } else {
            None
        };
        if count == 0 {
            return Ok(None);
        }
        let descriptors = (0..count)
            .map(|index| DescriptorInfo {
                index,
                number: FIRST_FD + index as i32,
                name: names.as_ref().map(|names| names[index].to_owned()),
            })
            .collect();
        Ok(Some(Metadata { descriptors }))
    }
}

#[cfg(unix)]
mod unix {
    use super::{DescriptorInfo, Metadata};
    use std::{
        error::Error,
        fmt, io,
        os::fd::{AsFd, FromRawFd, OwnedFd},
    };
    #[derive(Debug)]
    pub struct ActivatedFd {
        pub info: DescriptorInfo,
        pub handle: OwnedFd,
    }
    #[derive(Debug)]
    pub struct Activation {
        descriptors: Vec<ActivatedFd>,
    }
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum NameError {
        Missing(String),
        Ambiguous(String),
    }
    impl fmt::Display for NameError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "activation name selection error: {self:?}")
        }
    }
    impl Error for NameError {}
    impl Metadata {
        /// Take ownership of all inherited descriptors, then set each CLOEXEC.
        /// On a syscall failure all claimed owners are dropped, closing the entire
        /// batch. Original descriptor flags other than CLOEXEC are preserved.
        /// Socket/file type and application suitability still require validation.
        ///
        /// # Safety
        /// Every listed number must refer to a valid open descriptor in THIS
        /// process. The caller transfers exclusive Rust ownership of all of them;
        /// no other OwnedFd/File/socket wrapper may own these numbers. No concurrent
        /// close/replacement/claim is allowed. Claim at most once, normally directly
        /// after manager exec and before threads or libraries can own these slots.
        /// Successfully parsing env or probing fcntl does NOT prove these conditions.
        /// Also coordinate concurrent fork/exec until this returns: CLOEXEC is set
        /// per descriptor with fcntl, not atomically for the entire inherited batch.
        pub unsafe fn claim(self) -> io::Result<Activation> {
            let descriptors: Vec<_> = self
                .descriptors
                .into_iter()
                .map(|info| {
                    // SAFETY: caller promises each descriptor is valid and unowned.
                    let handle = unsafe { OwnedFd::from_raw_fd(info.number) };
                    ActivatedFd { info, handle }
                })
                .collect();
            for entry in &descriptors {
                let flags = rustix::io::fcntl_getfd(entry.handle.as_fd())?;
                rustix::io::fcntl_setfd(
                    entry.handle.as_fd(),
                    flags | rustix::io::FdFlags::CLOEXEC,
                )?;
            }
            Ok(Activation { descriptors })
        }
    }
    impl Activation {
        pub fn descriptors(&self) -> &[ActivatedFd] {
            &self.descriptors
        }
        pub fn named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a ActivatedFd> {
            self.descriptors
                .iter()
                .filter(move |entry| entry.info.name.as_deref() == Some(name))
        }
        /// Select by tag only when unique; error leaves every owner in this batch.
        pub fn take_unique(&mut self, name: &str) -> Result<ActivatedFd, NameError> {
            let mut matches = self
                .descriptors
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.info.name.as_deref() == Some(name));
            let index = matches
                .next()
                .map(|(index, _)| index)
                .ok_or_else(|| NameError::Missing(name.into()))?;
            if matches.next().is_some() {
                return Err(NameError::Ambiguous(name.into()));
            }
            Ok(self.descriptors.remove(index))
        }
        pub fn into_descriptors(self) -> Vec<ActivatedFd> {
            self.descriptors
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            io::Read,
            os::{fd::AsRawFd, unix::net::UnixStream},
        };
        #[test]
        fn name_errors_preserve_owners_and_unique_take_is_explicit() {
            let mut descriptors = Vec::new();
            let mut peers = Vec::new();
            for (index, name) in ["duplicate", "duplicate", "unique"].into_iter().enumerate() {
                let (socket, peer) = UnixStream::pair().unwrap();
                let info = DescriptorInfo {
                    index,
                    number: socket.as_raw_fd(),
                    name: Some(name.into()),
                };
                descriptors.push(ActivatedFd {
                    info,
                    handle: socket.into(),
                });
                peers.push(peer);
            }
            let mut activation = Activation { descriptors };
            assert_eq!(activation.named("duplicate").count(), 2);
            assert_eq!(
                activation.take_unique("duplicate").unwrap_err(),
                NameError::Ambiguous("duplicate".into())
            );
            assert_eq!(
                activation.take_unique("absent").unwrap_err(),
                NameError::Missing("absent".into())
            );
            assert_eq!(activation.descriptors().len(), 3);
            let selected = activation.take_unique("unique").unwrap();
            drop(activation);
            for peer in &mut peers[..2] {
                peer.set_nonblocking(true).unwrap();
                assert_eq!(peer.read(&mut [0]).unwrap(), 0);
            }
            peers[2].set_nonblocking(true).unwrap();
            assert_eq!(
                peers[2].read(&mut [0]).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            drop(selected);
            assert_eq!(peers[2].read(&mut [0]).unwrap(), 0);
        }
    }
}
#[cfg(unix)]
pub use unix::{ActivatedFd, Activation, NameError};

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(pid: &str, count: &str, names: Option<&str>) -> Snapshot {
        Snapshot {
            listen_pid: Some(pid.into()),
            listen_fds: Some(count.into()),
            listen_fdnames: names.map(Into::into),
        }
    }
    #[test]
    fn absent_foreign_and_zero_activation() {
        assert!(
            Snapshot::default()
                .parse(42, Limits::default())
                .unwrap()
                .is_none()
        );
        assert!(
            snapshot("41", "garbage", Some("bad\n"))
                .parse(42, Limits::default())
                .unwrap()
                .is_none()
        );
        assert!(
            snapshot("42", "0", None)
                .parse(42, Limits::default())
                .unwrap()
                .is_none()
        );
        assert!(
            snapshot("42", "0", Some(""))
                .parse(42, Limits::default())
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn tags_are_nonunique_and_absence_is_distinct() {
        let metadata = snapshot("42", "3", Some("listener:listener:unknown"))
            .parse(42, Limits::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            metadata.descriptors()[0].name,
            metadata.descriptors()[1].name
        );
        assert_eq!(metadata.descriptors()[2].number, 5);
        assert_eq!(metadata.descriptors()[2].name.as_deref(), Some("unknown"));
        assert!(
            snapshot("42", "2", None)
                .parse(42, Limits::default())
                .unwrap()
                .unwrap()
                .descriptors()
                .iter()
                .all(|entry| entry.name.is_none())
        );
        // Empty names are opaque tags, not a toolkit-generated identity.
        assert_eq!(
            snapshot("42", "2", Some(":listener"))
                .parse(42, Limits::default())
                .unwrap()
                .unwrap()
                .descriptors()[0]
                .name
                .as_deref(),
            Some("")
        );
    }
    #[test]
    fn malformed_inputs_are_separate_from_ownership() {
        let cases = [
            (snapshot("+42", "1", None), ParseError::InvalidPid),
            (snapshot("0", "1", None), ParseError::InvalidPid),
            (snapshot("4294967296", "1", None), ParseError::InvalidPid),
            (snapshot("42", "-1", None), ParseError::InvalidCount),
            (
                snapshot("42", "18446744073709551616", None),
                ParseError::CountOverflow,
            ),
            (
                snapshot("42", "2147483647", None),
                ParseError::DescriptorRangeOverflow,
            ),
            (snapshot("42", "4097", None), ParseError::TooManyDescriptors),
            (
                snapshot("42", "2", Some("one")),
                ParseError::NameCountMismatch,
            ),
            (snapshot("42", "1", Some("bad\n")), ParseError::InvalidNames),
            (
                Snapshot {
                    listen_pid: None,
                    listen_fds: Some("1".into()),
                    listen_fdnames: None,
                },
                ParseError::MissingPid,
            ),
            (
                Snapshot {
                    listen_pid: Some("42".into()),
                    listen_fds: None,
                    listen_fdnames: None,
                },
                ParseError::MissingCount,
            ),
        ];
        for (snapshot, expected) in cases {
            assert_eq!(snapshot.parse(42, Limits::default()).unwrap_err(), expected);
        }
        assert_eq!(
            snapshot("42", "1", Some(&"a".repeat(256)))
                .parse(42, Limits::default())
                .unwrap_err(),
            ParseError::InvalidNames
        );
        assert_eq!(
            Snapshot::default()
                .parse(
                    42,
                    Limits {
                        max_fds: MAX_FDS + 1
                    }
                )
                .unwrap_err(),
            ParseError::InvalidLimits
        );
    }
    #[test]
    fn segment_overcount_is_rejected_without_materializing_every_segment() {
        // 768 bytes is the longest names value the total-length pre-check
        // permits for count 3; as colons it forms 769 segments. Collection
        // stops after count + 1 entries before NameCountMismatch rejects it,
        // so work is bounded by the declared count, not the input length.
        let names = ":".repeat(3 * (MAX_NAME_BYTES + 1));
        assert_eq!(
            snapshot("42", "3", Some(&names))
                .parse(42, Limits::default())
                .unwrap_err(),
            ParseError::NameCountMismatch
        );
        // A single long tag declared as several slots likewise mismatches
        // by count; no per-separator allocation is involved.
        assert_eq!(
            snapshot("42", "4", Some(&"a".repeat(MAX_NAME_BYTES)))
                .parse(42, Limits::default())
                .unwrap_err(),
            ParseError::NameCountMismatch
        );
    }
    #[cfg(unix)]
    #[test]
    fn non_utf8_metadata_is_rejected_without_environment_mutation() {
        use std::os::unix::ffi::OsStringExt;
        let mut snapshot = snapshot("42", "1", None);
        snapshot.listen_fdnames = Some(OsString::from_vec(vec![0xff]));
        assert_eq!(
            snapshot.parse(42, Limits::default()).unwrap_err(),
            ParseError::InvalidNames
        );
    }
}
