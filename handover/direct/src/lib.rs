//! Separate, caller-supplied connected AF_UNIX SOCK_DGRAM control socket for one
//! generation/offer. Bundle transport remains custody-only. No launcher, executor,
//! application codec, memfd requirement or lifecycle state machine replaces core.
//! Sender uses ParkingController callbacks: publish bundle, wait_ready to verify
//! dormant custody, relinquish application authority, then release its ReleaseGrant.
//! abort returns retirement evidence; ONLY core may then issue an old ResumePermit.
//!
//! Caller supplies a fresh 128-bit session token, exclusive sockets/empty queues and
//! authenticated/trusted peer policy. This is correlation, not authentication.
//! Readiness attests validated/restored DORMANT application state. All I/O and shared
//! state side effects remain gated. Application-created copies/parked state stay
//! application-owned and must survive failure without auto-resume. Retirement success
//! must cover every such copy/actor, not merely the library Delivery. Rust tokens
//! cannot police aliases or a peer that lies about quiescence.
//!
//! Finite operation deadlines, no automatic retries. Errors after control send are
//! ambiguous and poison the session. RELEASE requires core's one-shot grant; lost
//! release/ACK never permits old resume. Receiver activation ACK failures retain
//! delivery AND received permission for an explicit decision. Stage/retirement errors
//! retain acquired delivery where it still exists. Drop only closes owned disposable
//! control/export refs, never sends I/O, resumes an application or cleans its copies.
//!
//! V1 wire frame: `HODIR01\0` magic (8), LE version u16, kind u8 (READY=1,
//! RELEASE=2, RELEASED=3, ABORT=4, RETIRED=5), zero u8, session bytes (16),
//! LE offer ID u64, LE generation length u16, zero u16, UTF-8 generation (<=256).
//! No body or ancillary FDs. All three correlation fields must match. This is
//! one-shot control: duplicate decisions cannot produce another received permission.
#![forbid(unsafe_code)]
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
