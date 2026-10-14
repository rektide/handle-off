//! `handle-off` — passing, retaining, and recovering access to open
//! resources, together with the information needed to continue using them.
//!
//! Stdio is one bundle; established network connections are another. Direct
//! descriptor passing, systemd fdstore custody, socket activation, and
//! cooperative state-carrying handover are independently useful capabilities
//! that compose without requiring every consumer to adopt the whole stack.
//!
//! All capabilities are opt-in. `local` enables only invocation and stdio;
//! defaults are empty. No codec, async runtime, or service manager is required.
//! `unix-rights` adds raw datagram descriptor transfer; `state-rkyv` adds a checked
//! metadata codec, and `state-memfd` adds an independent sealed Linux byte carrier.
//! `activation` parses classic manager metadata and explicitly claims inherited
//! descriptors without fdstore, codec, or bundle dependencies. `transport-unix`
//! selects the sample datagram bundle delivery/acceptance protocol. Manager custody
//! and cooperative replacement are not implemented yet.
//!
//! The current architecture proposal, its evidence, and the draft epic map
//! live in the repository under `design/fd-handoff/`.

#[cfg(feature = "bundle")]
pub use handle_off_bundle as bundle;
#[cfg(feature = "handles")]
pub use handle_off_handles as handles;
#[cfg(feature = "invocation")]
pub use handle_off_invocation as invocation;

#[cfg(feature = "state")]
pub use handle_off_state as state;
#[cfg(feature = "stdio")]
pub use handle_off_stdio as stdio;
#[cfg(feature = "unix-rights")]
pub use handle_off_unix_rights as unix_rights;

#[cfg(feature = "state-memfd")]
pub use handle_off_state_memfd as state_memfd;

#[cfg(feature = "activation")]
pub use handle_off_activation as activation;
#[cfg(feature = "state-rkyv")]
pub use handle_off_state_rkyv as state_rkyv;

#[cfg(feature = "transport-unix")]
pub use handle_off_transport_unix as transport_unix;
