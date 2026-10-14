//! `handle-off` — passing, retaining, and recovering access to open
//! resources, together with the information needed to continue using them.
//!
//! Stdio is one bundle; established network connections are another. Direct
//! descriptor passing, systemd fdstore custody, socket activation, and
//! cooperative state-carrying handover are independently useful capabilities
//! that compose without requiring every consumer to adopt the whole stack.
//!
//! This crate is the toolkit's facade anchor. Capability crates (descriptor
//! mechanics, bundles, state carriers, activation, custody, handover) attach
//! here as their contracts land. This is a fresh foundation: no public API is
//! settled yet, and the crate is intentionally empty.
//!
//! The current architecture proposal, its evidence, and the draft epic map
//! live in the repository under `design/fd-handoff/`.
