---
type: ImplementationReport
title: Rendezvous endpoint contract and sample explicit/env/XDG/local policy
description: Checkpoint record of the E2.4 slice covering the std-only handle-off-rendezvous contract crate, the handle-off-rendezvous-local sample policy with first-yield termination and predictable fail-versus-none semantics, standalone pinned-nightly verification while workspace integration is deferred, and pfd-handle-off-direct-rendezvous acceptance accounting.
resource: /doc/rendezvous/implementation0.glm53max.md
status: implemented
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6d652c3ffespR5CgFSQ5uwz6"
    title: "Implemented both rendezvous crates, their tests, this report, and the standalone pinned-nightly validation under .test-agent/rendezvous"
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E2.4 acceptance criteria and the rendezvous crate-family boundary
  - resource: /design/fd-handoff/work.md
    title: Checkpoint coordination, toolchain policy, and E2.3 transport context
  - resource: /transport/unix/src/lib.rs
    title: Already-connected datagram consumer whose callers bypass resolution
  - resource: /rendezvous/core/Cargo.toml
    title: Std-only manifest awaiting workspace membership
  - resource: /rendezvous/core/src/lib.rs
    title: Contract crate docs: candidates are unverified, bypass is legitimate
  - resource: /rendezvous/core/src/unix.rs
    title: Endpoint, AbstractName, ServiceName, Origin, Candidate, ResolveError, Resolver, Environment
  - resource: /rendezvous/local/Cargo.toml
    title: Sample policy manifest with a single path dependency
  - resource: /rendezvous/local/src/lib.rs
    title: Source-semantics table and first-yield termination rule
  - resource: /rendezvous/local/src/unix.rs
    title: LocalPolicy implementation and twelve policy tests
  - resource: /handle-off/Cargo.toml
    title: Facade integration point owned by the concurrent slice
  - resource: /Cargo.toml
    title: Workspace membership integration point owned by the concurrent slice
---

# Rendezvous endpoint contract and sample explicit/env/XDG/local policy

Two new std-only crates deliver E2.4 of the
[architecture](/design/fd-handoff/architecture0.gpt6a.md): `handle-off-rendezvous`
(`rendezvous/core`) is the endpoint-resolution contract and data; `handle-off-rendezvous-local`
(`rendezvous/local`) is one concrete, replaceable explicit/environment/XDG/local policy
over it. Neither crate connects, launches, authenticates, or removes anything, and a
caller that already holds a connected backend — the transport-unix datagram endpoints of
E2.3, for example — bypasses resolution entirely. The rendezvous crates were kept
deliberately decoupled from E2.3's protocol: they resolve addresses for any consumer and
depend on no transport, bundle, or state crate.

This report was written by the implementing session
(`opencode://ses_ec6d652c3ffespR5CgFSQ5uwz6`). Workspace integration is **not** included:
the root `Cargo.toml`, `Cargo.lock`, facade, and README are concurrently owned by the
systemd/facade slice, so the crates are validated standalone (below) and the exact
integration steps are recorded for the parent.

## The contract crate: `handle-off-rendezvous`

- **`Endpoint`** — a Unix-domain address, either a filesystem `Path` (native bytes
  preserved; non-UTF-8 is fine) or an `Abstract` name. Abstract representation is
  justified by the manager ecosystem this toolkit targets: systemd socket units name
  abstract sockets directly (`ListenStream=@…`, `ListenDatagram=@…`), and abstract
  endpoints leave no stale socket file behind, which suits this toolkit's deliberate
  no-deletion stance. `Endpoint::socket_addr()` converts for a caller about to connect
  (no I/O): over-long paths surface std's `SUN_LEN` error and abstract conversion needs
  Linux, returning `Unsupported` elsewhere. There is no universal transport taxonomy:
  an endpoint does not claim a socket type, and other address families would be separate
  kinds, not options bolted onto this one.
- **`AbstractName`** — nonempty, no interior NUL, deliberately stricter than the kernel
  (std's Linux `SocketAddrExt` accepts both; verified by probe under
  `.test-agent/rendezvous/`). The restriction keeps every abstract name round-trippable
  through the NUL-free text convention.
- **`Endpoint::parse_value`** — the shared text convention for environment values and
  CLI arguments: leading `@` selects abstract (systemd style), anything else is a
  filesystem path, byte-for-byte. A path genuinely starting with `@` has no text form;
  construct it typed.
- **`ServiceName`** / **`InvalidName`** — caller-owned names validated once: nonempty,
  ≤255 bytes, no `/`, no ASCII control characters (NUL included). A validated name
  embedded between a prefix and suffix can never form a `.`/`..` or separator component.
- **`Origin`**, **`Candidate`** — a candidate is one unverified endpoint plus a
  free-form provenance label (`explicit`, `environment:VAR`, `xdg-runtime-dir`,
  `local-dir`). Candidates assert configuration, never reachability.
- **`ResolveError`** (`#[non_exhaustive]`) — `Name(InvalidName)` and
  `Endpoint { origin, invalid }`. Invalid input data fails predictably; absent
  configuration is not an error.
- **`Resolver`** — an object-safe trait returning ordered unverified candidates. An
  empty result means "nothing configured", and what to do then — including the opt-in
  short-lived server of E9 — is deliberately caller policy.
- **`Environment`** — a read-only snapshot used as resolution input. `capture()` copies
  the current environment once (a convenience, not a coordination mechanism); lookups
  are exact native-byte, first match wins; nothing ever mutates the live environment.

## The sample policy: `handle-off-rendezvous-local`

`LocalPolicy::new(namespace, environment_snapshot)` plus simple configuration methods
(`with_endpoint`, `with_endpoint_variable`, `with_local_directory`, `with_order`)
implements `Resolver`. The application owns every name: namespace, service name,
environment variable name, and base directory. Nothing is branded — no fixed variable
name, socket directory, or daemon identity — beyond the `XDG_RUNTIME_DIR` convention
itself and the flat `<base>/<namespace>-<name>.sock` candidate layout (validated names
cannot escape the base; directories are never created, forced, or verified).

**First-yield termination is the anti-fallback rule.** Sources are consulted in the
configured order (`DEFAULT_ORDER`: explicit, environment, XDG, local) and the first
source that yields a candidate ends resolution; later sources are neither read nor
validated. Candidates from multiple configured sources are never mixed, so a failed
connection cannot silently proceed to a differently-configured server. With the default
order an explicit endpoint therefore wins outright — an invalid *explicit text* fails
earlier still, at `Endpoint::parse_value`, immediately and never silently. The order is
used verbatim (empty disables all sources; duplicates yield at first occurrence).

Fail-versus-none semantics per source (also in the crate docs):

| Source | Yields | Errors | Contributes nothing |
| --- | --- | --- | --- |
| Explicit | endpoint configured | never at resolution time (typed endpoints are valid by construction; invalid text failed at parse) | no endpoint |
| Environment | variable present in snapshot | present but unparseable (empty, bare `@`, interior NUL) | variable unconfigured or absent |
| XdgRuntimeDir | `XDG_RUNTIME_DIR` nonempty | value contains interior NUL | unset or empty (XDG treats empty as unset) |
| LocalDir | base configured | base contains interior NUL | not configured |

**Candidates are not verified servers.** Resolution performs no `stat`: a filesystem
path candidate may not exist, an existing socket file may be stale with no listener, and
abstract candidates never exist as files. Connecting, checking liveness, and
interpreting failure are caller-owned. Resolution never reads the live environment
(snapshots only) and never deletes sockets, starts servers, or authenticates.

## Verification

Toolchain: the repo-pinned mise-selected `nightly-2026-09-30` (`rustc 1.101.0-nightly`,
`5c543b0b8`). Because the crates are not yet workspace members, everything was checked
with direct `rustc`/`rustdoc`/`clippy-driver` invocations, recorded reproducibly in
[.test-agent/rendezvous/README.md](/.test-agent/rendezvous/README.md) (scratch,
gitignored). Passing at delivery time:

- `handle-off-rendezvous`: 9 unit tests + 1 doctest.
- `handle-off-rendezvous-local`: 12 unit tests + 1 doctest, compiled against the
  path-built core rlib.
- clippy (`-D warnings`) clean on lib and test targets of both crates; `rustfmt
  --check` clean.

Test coverage is policy-meaningful, not line-padding: default order (XDG preferred over
a shared local directory), verbatim reordering, explicit short-circuit *without
consulting* an invalid environment value, both endpoint forms plus non-UTF-8 values and
variable names, present-but-invalid environment values erroring instead of falling
through to `/tmp`, absent variable falling through, empty `XDG_RUNTIME_DIR` behaving as
unset, nothing-configured yielding `Ok(vec![])`, namespace/name validation and flat
filename embedding, interior-NUL directory inputs erroring predictably, and `&dyn
Resolver` object-safety. No test mutates the environment; all snapshots are
constructed. An explicit-failure gap is intentionally not papered over: an invalid
explicit endpoint cannot be represented in a `LocalPolicy` at all.

Deferred to workspace integration (parent/Sol, after membership lands): `cargo
nextest` under the workspace, `cargo doc`, facade feature graph compilation,
`Cargo.lock` regeneration, and README exposure.

## Integration required (owned by parent / the concurrent slice)

1. Root `Cargo.toml` members: append `"rendezvous/core"`, `"rendezvous/local"`.
   The manifests already inherit `version`/`edition`/`authors`/`license` from the
   workspace; `handle-off-rendezvous-local` declares the single path dependency on
   `handle-off-rendezvous`.
2. Facade `handle-off/Cargo.toml`: add optional dependencies
   `handle-off-rendezvous = { path = "../rendezvous/core", version = "0.1.0", optional = true }`
   and `handle-off-rendezvous-local = { path = "../rendezvous/local", version = "0.1.0", optional = true }`,
   plus features `rendezvous = ["dep:handle-off-rendezvous"]` and
   `rendezvous-local = ["dep:handle-off-rendezvous-local", "rendezvous"]`. Keep `local`
   exactly `["invocation", "stdio"]`.
3. Facade `src/lib.rs`: reexport as `pub use handle_off_rendezvous as rendezvous;` /
   `pub use handle_off_rendezvous_local as rendezvous_local;` behind the features, and
   extend the facade crate-doc feature enumeration.
4. `Cargo.lock`: regenerated by the first cargo invocation — two new path packages, no
   new registry entries (both crates are std-only).
5. README exposure: a short Rendezvous section near Bundle delivery, stating the
   unverified-candidates and never-starts-a-server boundaries.
6. Consumers (E5.x CLI): accept `Option<&dyn Resolver>`; a caller-supplied connected
   `UnixDatagram` bypasses resolution entirely — that path needs no rendezvous feature.
   Connect via `Endpoint::socket_addr()`.

## Acceptance accounting: pfd-handle-off-direct-rendezvous

Acceptance criteria (architecture E2.4): "Explicit/env/local/XDG resolver with
caller-owned names; connected-endpoint injection bypasses resolution; discovery never
silently starts a server."

- **Explicit/env/local/XDG resolver with caller-owned names** — `LocalPolicy` supplies
  exactly these four sources; namespace, service names, variable names, and base
  directories are all caller-owned; order is configurable with a documented default.
- **Connected-endpoint injection bypasses resolution** — the bypass needs no API: the
  transport takes already-connected sockets, and callers simply do not construct a
  resolver. Both crates' docs state this; the crates depend on no transport.
- **Discovery never silently starts a server** — no code path launches anything; an
  empty candidate list is returned and the E9 opt-in server is documented as caller
  policy outside the crates.

Ticket status at documentation time: `open`, awaiting parent integration and the
focused review. Tracker updates remain parent-owned.

## Links

- Direction and acceptance criteria:
  [architecture — E2.4](/design/fd-handoff/architecture0.gpt6a.md#e2--descriptor-transport-and-direct-bundle-delivery)
- Coordination: [work map](/design/fd-handoff/work.md)
- Consumer that bypasses resolution: [transport report](/doc/transport/rights0.glm53max.md)
- Adjacent manager-side slice: [activation report](/doc/systemd/activation0.glm53max.md)
