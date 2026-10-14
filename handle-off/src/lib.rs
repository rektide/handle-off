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
//! Transport, custody, and cooperative replacement are not implemented yet.
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
