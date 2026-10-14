//! Sample rendezvous policy: explicit endpoint, environment variable,
//! XDG runtime directory, local directory.
//!
//! This is one concrete, replaceable policy over the
//! `handle-off-rendezvous` contract, not a required policy and not a
//! registry. The application owns every name: the namespace, the service
//! name resolved at call time, the environment variable that may carry an
//! endpoint, and any local base directory. No variable name, socket
//! directory, suffix, or daemon identity is baked in.
//!
//! Sources are consulted in the configured order ([`DEFAULT_ORDER`] unless
//! overridden). **The first source that yields a candidate terminates
//! resolution**; sources after it are neither read nor validated. Candidates
//! from multiple configured sources are never mixed, so a failed connection
//! attempt cannot silently proceed to a differently-configured server. With
//! the default order an explicitly configured endpoint therefore wins
//! outright, and no later source is even looked at.
//!
//! Each source's outcome is documented so fail-versus-none stays predictable:
//!
//! | Source | Yields | Errors | Contributes nothing |
//! | --- | --- | --- | --- |
//! | [`Source::Explicit`] | an endpoint is configured | never at resolution time — a typed [`Endpoint`] was validated when constructed, so invalid explicit *text* fails at `Endpoint::parse_value`, immediately and never silently | no endpoint configured |
//! | [`Source::Environment`] | the configured variable is present in the snapshot | present but unparseable (empty, bare `@`, interior NUL) — never silently skipped | variable not configured, or absent from the snapshot |
//! | [`Source::XdgRuntimeDir`] | `XDG_RUNTIME_DIR` is set and nonempty | the value contains an interior NUL byte | unset or empty (the XDG convention treats empty as unset) |
//! | [`Source::LocalDir`] | a base directory is configured | the base contains an interior NUL byte | no base configured |
//!
//! Derived filesystem candidates are flat components
//! `<base>/<namespace>-<name>.sock` — validated names cannot contain `/`, so
//! they cannot escape the base — and are **unverified**: no `stat`, connect,
//! or server launch happens, and an existing socket file would still not
//! prove a live server. An empty result (nothing configured anywhere) is not
//! an error; what to do then, including any opt-in short-lived server, is
//! caller policy outside this crate. Relative base directories stay
//! relative and resolve against the connecting process's cwd.
//!
//! The environment is consumed only as a caller-supplied
//! [`Environment`] snapshot; resolution never reads or mutates the live
//! process environment.
#![forbid(unsafe_code)]
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
