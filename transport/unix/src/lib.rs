//! Optional concrete sample backend, not a universal transport abstraction.
//!
//! Supply fresh, already-connected UnixDatagram endpoints with empty receive
//! queues and exclusive protocol use. Socket clones/raw I/O must not interleave.
//! Exchanges are serialized, half-duplex and single-in-flight; simultaneous offers
//! by both peers are not supported. No discovery, authentication or runtime policy.
//! Configure timeouts/nonblocking mode before construction. Default blocking I/O
//! can wait indefinitely; errors are exposed, with no automatic retry.
//!
//! V1 frame: 8-byte `HOUDG\0\0\0` magic, LE u16 version, u8 kind (offer=1,
//! accepted=2, rejected=3), zero reserved byte, LE u64 offer ID, LE u16 generation
//! byte length, LE u16 FD count, LE u32 body length, then UTF-8 generation and body.
//! Offers contain one checked rkyv manifest; ancillary slot i is exactly
//! manifest.resources\[i\]. ACK/NACK carry neither body nor FDs. Resource order is
//! only a delivery binding, not FD/OFD identity. Duplicate resource IDs are invalid;
//! aliases use repeated member references to the same resource ID instead.
//!
//! IDs increase for each local offer and ACKs echo both ID and generation. Never
//! rewrap a used socket as a fresh endpoint: that resets correlation history.
//! Receiver acceptance follows structural validation and deliberate application
//! review. It is custody acceptance, NOT command completion, handover commit,
//! authentication, revocation of originals, or permission for simultaneous I/O.
//! Sender keeps every original descriptor. Any failure after attempting to submit
//! an offer is conservatively indeterminate, unless a correlated NACK was received.
//! The failed endpoint becomes unusable; replacement/recovery is caller-owned.
//! A lost ACK cannot undo the receiver's acceptance. Accept errors preserve the
//! delivery's owned references for an explicit application decision.
#![forbid(unsafe_code)]
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
