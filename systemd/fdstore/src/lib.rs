//! Named FDSTORE/FDSTOREREMOVE operations, not generation or replay coordination.
//! Successful submission and barrier ordering do not establish acceptance, capacity
//! or completed removal. Original references remain caller-owned. Names are tags:
//! one message applies one name to ALL its FDs; duplicate/nonunique names are valid.
//! Removing a tag targets all matching manager entries, not just one actor's offer.
//! Different names require separate, non-atomic messages. Manager deduplication,
//! polling and service-store lifetime are manager policy. No automatic prune.
//! Application-provided inspection is explicit; no D-Bus/busctl dependency or
//! native inspection adapter is included. FDSTORE requires manager support
//! (systemd >=219), removal >=236, and FDPOLL/BARRIER >=246. These are not negotiated
//! by this primitive. Receipt verification is only as strong
//! as the supplied verifier's unit, identity, capacity and permission checks.
#![forbid(unsafe_code)]
use std::{error::Error, ffi::OsString, fmt};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FdName(String);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidName;
impl fmt::Display for InvalidName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FDNAME must be <=255 ASCII bytes, excluding controls and ':'")
    }
}
impl Error for InvalidName {}
impl FdName {
    /// Matches systemd fdname_is_valid: empty strings are explicitly permitted,
    /// as are spaces, '=' and reserved tags. This does not create a unique identity.
    pub fn new(name: impl Into<String>) -> Result<Self, InvalidName> {
        let name = name.into();
        if name.len() > 255 || name.bytes().any(|b| !(32..127).contains(&b) || b == b':') {
            return Err(InvalidName);
        }
        Ok(Self(name))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Support {
    #[default]
    Unknown,
    Supported,
    Unsupported,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub store: Support,
    pub remove: Support,
    pub poll_control: Support,
    pub inspection: Support,
    pub barrier: Support,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capacity {
    pub maximum: u64,
    pub occupied: u64,
}
impl Capacity {
    /// Snapshot arithmetic, not a reservation or prediction of manager deduplication.
    pub fn can_fit(&self, additional: u64) -> bool {
        self.occupied
            .checked_add(additional)
            .is_some_and(|needed| needed <= self.maximum)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ManagerReport {
    pub capabilities: Capabilities,
    pub capacity: Option<Capacity>,
}
#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub fdstore: Option<OsString>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidCapacityHint;
impl fmt::Display for InvalidCapacityHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FDSTORE environment capacity must be an unsigned integer")
    }
}
impl Error for InvalidCapacityHint {}
impl Environment {
    pub fn capture() -> Self {
        Self {
            fdstore: std::env::var_os("FDSTORE"),
        }
    }
    /// Reported startup maximum (systemd >=254), not current occupancy or evidence
    /// of acceptance. Absence is unknown, not proof that older managers lack fdstore.
    pub fn maximum_hint(&self) -> Result<Option<u64>, InvalidCapacityHint> {
        let Some(value) = &self.fdstore else {
            return Ok(None);
        };
        let value = value.to_str().ok_or(InvalidCapacityHint)?;
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(InvalidCapacityHint);
        }
        value.parse().map(Some).map_err(|_| InvalidCapacityHint)
    }
}

#[cfg(unix)]
mod unix {
    use super::FdName;
    use handle_off_systemd_notify::{Notifier, Submission};
    use std::{io, os::fd::BorrowedFd};
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum PollPolicy {
        /// Omit FDPOLL; manager default removes pollable entries on HUP/ERR.
        ManagerDefault,
        /// Emit FDPOLL=0; explicit removal/lifetime management remains application-owned.
        Disabled,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Verification {
        NoManager,
        Unverified,
        Unsupported,
        Confirmed,
        Rejected,
    }
    #[derive(Debug)]
    pub struct StoreReceipt<'fd> {
        pub name: FdName,
        /// Borrowed original references remain available to an explicit inspector.
        pub descriptors: Vec<BorrowedFd<'fd>>,
        pub submission: Submission,
    }
    impl StoreReceipt<'_> {
        /// No built-in identity claim. An application adapter must inspect the right
        /// unit and compare the intended resources, not merely count or raw FD numbers.
        pub fn verify_with<E>(
            &self,
            verifier: impl FnOnce(&Self) -> Result<Verification, E>,
        ) -> Result<Verification, E> {
            if self.submission == Submission::NoManager {
                return Ok(Verification::NoManager);
            }
            verifier(self)
        }
    }
    #[derive(Debug)]
    pub struct RemoveReceipt {
        pub name: FdName,
        pub submission: Submission,
    }
    impl RemoveReceipt {
        pub fn verify_with<E>(
            &self,
            verifier: impl FnOnce(&Self) -> Result<Verification, E>,
        ) -> Result<Verification, E> {
            if self.submission == Submission::NoManager {
                return Ok(Verification::NoManager);
            }
            verifier(self)
        }
    }
    pub struct Store<'a> {
        notifier: &'a Notifier,
    }
    impl<'a> Store<'a> {
        pub fn new(notifier: &'a Notifier) -> Self {
            Self { notifier }
        }
        pub fn offer<'fd>(
            &self,
            name: &FdName,
            descriptors: &[BorrowedFd<'fd>],
            poll: PollPolicy,
        ) -> io::Result<StoreReceipt<'fd>> {
            if descriptors.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "fdstore offer must contain at least one descriptor",
                ));
            }
            let message = match poll {
                PollPolicy::ManagerDefault => format!("FDSTORE=1\nFDNAME={}", name.as_str()),
                PollPolicy::Disabled => format!("FDSTORE=1\nFDNAME={}\nFDPOLL=0", name.as_str()),
            };
            let submission = self.notifier.send(&message, descriptors)?;
            Ok(StoreReceipt {
                name: name.clone(),
                descriptors: descriptors.to_vec(),
                submission,
            })
        }
        /// Explicit removal of ALL entries carrying this tag. Submission does not
        /// prove removal; verify separately before resuming any state tied to custody.
        pub fn remove(&self, name: &FdName) -> io::Result<RemoveReceipt> {
            let submission = self
                .notifier
                .send(&format!("FDSTOREREMOVE=1\nFDNAME={}", name.as_str()), &[])?;
            Ok(RemoveReceipt {
                name: name.clone(),
                submission,
            })
        }
    }
}
#[cfg(unix)]
pub use unix::*;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_capacity_follow_protocol_not_identity_assumptions() {
        for valid in [
            "",
            "stored",
            "unknown",
            "connection",
            "name with spaces=ok",
            "!@#$%^&*",
        ] {
            assert!(FdName::new(valid).is_ok());
        }
        for invalid in ["colon:name", "line\n", "tab\t", "nul\0", "\u{7f}", "é"] {
            assert!(FdName::new(invalid).is_err());
        }
        assert!(FdName::new("x".repeat(255)).is_ok());
        assert!(FdName::new("x".repeat(256)).is_err());
        assert!(
            Capacity {
                maximum: 4,
                occupied: 2
            }
            .can_fit(2)
        );
        assert!(
            !Capacity {
                maximum: 4,
                occupied: 2
            }
            .can_fit(3)
        );
        assert!(
            !Capacity {
                maximum: u64::MAX,
                occupied: u64::MAX
            }
            .can_fit(1)
        );
        assert!(
            !Capacity {
                maximum: 1,
                occupied: 2
            }
            .can_fit(0)
        );
        assert_eq!(
            ManagerReport::default().capabilities.inspection,
            Support::Unknown
        );
    }
    #[test]
    fn capacity_environment_is_only_a_reported_hint() {
        assert_eq!(Environment::default().maximum_hint(), Ok(None));
        assert_eq!(
            Environment {
                fdstore: Some("2048".into())
            }
            .maximum_hint(),
            Ok(Some(2048))
        );
        for value in ["", "-1", "+1", " 1", "18446744073709551616"] {
            assert!(
                Environment {
                    fdstore: Some(value.into())
                }
                .maximum_hint()
                .is_err()
            );
        }
    }
}
