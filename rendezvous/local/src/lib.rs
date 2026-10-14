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
//! | [`Source::Explicit`] | an endpoint is configured | never at resolution time — a typed [`Endpoint`](handle_off_rendezvous::Endpoint) was validated when constructed, so invalid explicit *text* fails at [`Endpoint::parse_value`](handle_off_rendezvous::Endpoint::parse_value), immediately and never silently | no endpoint configured |
//! | [`Source::Environment`] | the configured variable is present in the snapshot | present but unparseable, **including an empty value** (also bare `@`, interior NUL) — never silently skipped; to disable the source, remove the variable from the environment instead of emptying it | variable not configured, or absent from the snapshot |
//! | [`Source::XdgRuntimeDir`] | `XDG_RUNTIME_DIR` is set and nonempty | the value contains an interior NUL byte | unset or empty (the XDG convention treats empty as unset, unlike the environment source above) |
//! | [`Source::LocalDir`] | a base directory is configured | the base contains an interior NUL byte | no base configured |
//!
//! Directory inputs (`with_local_directory`, `XDG_RUNTIME_DIR`) are not
//! validated at configuration time: an interior NUL surfaces only if and
//! when its source is actually consulted and yields, per the first-yield
//! rule — an earlier yielding source means it is never examined at all.
//!
//! Derived filesystem candidates are flat components
//! `<base>/<namespace-bytes>-<namespace>-<name>.sock`, where the leading
//! number is the UTF-8 byte length of the validated namespace. The prefix
//! makes the scheme injective — the filename determines the
//! (namespace, name) pair, so pairs like `a`/`b-c` and `a-b`/`c` cannot
//! collide as they did under a plain `<namespace>-<name>.sock` scheme.
//! Validated names cannot contain `/`, so candidates cannot escape the
//! base; base directories are caller-owned and are never created. The
//! per-name byte limit does not guarantee that the composed filename fits
//! the filesystem component limit or that the full path fits a Unix socket
//! address; these are separate limits checked when using the candidate.
//! The
//! candidates are **unverified**: no `stat`, connect, or server launch
//! happens, and an existing socket file would still not prove a live
//! server. An empty result (nothing configured anywhere) is not an error;
//! what to do then, including any opt-in short-lived server, is caller
//! policy outside this crate. Relative base directories stay relative and
//! resolve against the connecting process's cwd.
//!
//! The environment is consumed only as a caller-supplied
//! [`Environment`](handle_off_rendezvous::Environment) snapshot; resolution never reads or mutates the live
//! process environment.
#![forbid(unsafe_code)]
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
