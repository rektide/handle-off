//! Rendezvous endpoint resolution: contract and data only.
//!
//! This crate describes *where a server has been configured to be reachable*.
//! It never connects, launches, authenticates, or removes anything: a
//! [`Resolver`] derives ordered, **unverified** [`Candidate`] endpoints from
//! caller-supplied configuration and inputs. A filesystem-path candidate is
//! not evidence that a socket file exists, an existing socket file is not
//! evidence of a live server, and abstract-namespace candidates never exist
//! as files at all. Connection attempts, liveness checks, and their failures
//! are caller-owned.
//!
//! Callers that already hold a connected backend (for example a connected
//! `UnixDatagram` pair supplied to a direct transport) bypass resolution
//! entirely; nothing here is required or consulted on that path.
//!
//! [`Endpoint`] covers the two Unix-domain address kinds relevant to the
//! manager ecosystem this toolkit targets: filesystem paths and the Linux
//! abstract namespace. systemd socket units address abstract sockets as
//! `@name` (`ListenStream=@…`, `ListenDatagram=@…`), and servers that must
//! not leave stale socket files behind — this toolkit deliberately does not
//! delete socket files — can avoid the problem with abstract names. An
//! endpoint does not claim a socket type: the same address can back a stream
//! or datagram socket, and the chosen protocol decides suitability. There is
//! no universal transport taxonomy here; other address families would be
//! separate endpoint kinds, not options of this one.
#![forbid(unsafe_code)]
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
