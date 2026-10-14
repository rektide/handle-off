//! Optional Linux busctl adapter. No notify/fdstore operation, automatic pruning,
//! supervisor or generation lifecycle. Unit name, expected MainPID and user/system
//! bus are explicit. Queries use GetUnit (not LoadUnit), disable activation and
//! interactive authorization, and validate canonical unit/InvocationID/MainPID
//! before and after the snapshot. This is not an atomic capacity reservation.
//! Store/remove/poll/barrier support uses documented systemd version thresholds
//! (219/236/246/246); vendor backports and permissions are not negotiated. Inspection
//! support uses the actual method schema (upstream added it in 254), not version
//! alone. Absent capacity properties remain unknown, never invented from version.
//!
//! Membership means name plus fstat inode/device/mode metadata matches retained
//! originals, not equality of process-local FD numbers or proof of OFD identity.
//! Separate opens of the same inode can match; alias multiplicity cannot be proved.
//! Paths and open-status flags are diagnostic, not used as inode identity proofs.
//! Distinct objects under one tag are supported; unexpected identities and duplicate
//! indistinguishable rows fail closed. No acceptance is inferred from a barrier.
//!
//! The trusted busctl executable is optional at runtime. Output and aggregate query
//! time are bounded. Timeout/error kills its dedicated process group and reaps the
//! direct child, with no reader threads. Escaped malicious descendants are outside
//! this trusted-command contract. Kernel scheduling/unkillable tasks are not a
//! hard-real-time guarantee. Success payloads are typed JSON. busctl errors are
//! unstructured: recognized C-locale denial/unsupported messages are classified;
//! other failures remain explicit command errors, never guessed as missing data.
#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
mod data;
#[cfg(target_os = "linux")]
mod dump;
#[cfg(target_os = "linux")]
mod inspector;
#[cfg(target_os = "linux")]
mod runner;
#[cfg(target_os = "linux")]
pub use data::*;
#[cfg(target_os = "linux")]
pub use inspector::*;
