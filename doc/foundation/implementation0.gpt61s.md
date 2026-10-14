---
type: ImplementationReport
title: Handle-off decomposed foundation
description: Implemented resource and state associations, explicit local invocation and stdio, extraction owners, dependency graph and measured local baseline.
resource: /doc/foundation/implementation0.gpt61s.md
status: implemented
generated: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Read-only contract preparation followed by authorized decomposed foundation implementation and local verification
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: Authorized capability boundaries and initial acceptance criteria
  - resource: /design/fd-handoff/work.md
    title: Native work mapping and parent ownership of tracker updates
  - resource: /design/fd-handoff/research1.glm53max.md
    title: Recovered deployed bus-proxy handover and attributed prior evidence
  - resource: /REVIEW-0.md
    title: Retained legacy coupling, ownership and test hazards
  - resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/components/gateway/src/handover.rs"
    title: Actual application image, import membership checks and parked-owner lease inspected
  - resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/cli/daemon/src/handover.rs"
    title: Actual verified systemd transaction and positional sealed-image reads inspected
  - resource: "opencode://ses_ec6088918ffeIq7D1VJbVIsyA5"
    title: "Ticket prefix rework to handoff IDs; bus-proxy ticket IDs normalized to busprox"
---

# Decomposed foundation

This slice implements foundation contracts and local invocation/stdio, not
descriptor transfer or restart continuity. No external bus-proxy files changed.
The parent owns native tracker updates and the design corpus.

## Owners and extraction map

| Evidence/mechanism | Toolkit owner or retained application responsibility |
| --- | --- |
| Legacy clid `ExecutionContext` (source read from version history) | `invocation/core`: replace UTF-8-only strings and mandatory rkyv with native `OsString`/`PathBuf` |
| Legacy fixed stdio transfer and unsafe FD conversions recorded in review | `handles/stdio`: safe borrowed/owned roles; later Unix-rights crate owns actual transfer |
| Gateway `Image::import`, lines 235–387: named completeness before I/O | `handles/bundle`: metadata membership; application still checks actual socket kind/endpoints and state |
| Gateway `Frozen`/`Parked`, lines 390–446: lend copies while owners remain parked | `handles/core`: explicit duplication; later coordinator owns park/abort/commit permission |
| Gateway source framing/decoder, writer byte offsets, router/subscriptions and same-boot deadlines | Bus-proxy opaque extensions, not toolkit schema; retain JSON/base64 initially |
| Daemon transaction, lines 193–235: verify store name AND kernel identity, invalidate before old-owner resume | Later systemd custody/coordinator adapters; never infer acceptance from notify/barrier success |
| Daemon sealed image/read, lines 284–316: bounded positional read and seals | Later state-memfd carrier; payload format remains independently selected |

The reviewed legacy client/daemon behavior comes from retained review/research,
not a claim that this session re-read all their retired implementation. Bus-proxy
live-test claims remain attributed to research1; no live handover ran here.

## APIs and semantic limits

- `handle-off-handles`: checked `ResourceId`; `HandleTable<H>` with
  `OwnedHandles`/`BorrowedHandles`, collision-preserving `insert`, `get`, `remove`,
  `iter`, `borrow`, and explicit fallible `try_clone_to_owned`. Owned tables are
  not implicitly cloneable. Closing copies does not call socket shutdown.
- `handle-off-state`: `StateFormat { name, version }`, `Extension { key, format,
  required, payload }`, and `Payload::Inline`/`ResourceRange`. Structural checks
  reject invalid keys/formats and overflowing ranges. Carrier bounds/seals and
  application compatibility are not certified by these checks.
- `handle-off-bundle`: distinct generation, bundle and scoped member ID types;
  `Manifest { version, generation, resources, bundles, extensions }`, with
  extensions also attached to bundles/members. `validate` rejects duplicate IDs,
  duplicate per-scope extension keys, missing and unreferenced inventory entries.
  `validate_handles` requires exact inventory independent of transport order.
- `handle-off-invocation`: explicit `Invocation { argv, environment, cwd }` plus
  `capture_current`. argv includes argv[0], environment is a full snapshot rather
  than overlay. Capturing never parses, executes, changes cwd or mutates env.
- `handle-off-stdio`: generic `Stdio<H>` and owned/borrowed aliases;
  `BorrowedStdio::new` borrows caller-owned `AsFd` values, and explicit cloning
  allows safe conversion to `File` for local I/O. Role constants are conventional
  member names, not systemd FDNAME or transport positions.

Repeated members can share one ResourceId, including across bundles. This is a
declaration, not discovered OFD identity. Distinct resource IDs may also point to
one OFD. Raw FD numbers, ancillary positions, activation replay order and manager
names are adapter bindings and do not enter persistent role identity. Shared file
offsets/status flags remain shared after duplication.

The manifest version is semantic schema version 1; no wire envelope is selected.
No transport, lifecycle, capture/restore or codec trait framework was introduced.
Applications must recognize required extensions before resumed I/O; unknown
optional extensions can remain opaque. Structural validation intentionally does
not decide format support. Cross-bundle application graphs belong in generation
extensions. Multiple state ranges can reference shared or separate carriers.

Environment snapshots may contain credentials and are not atomic with concurrent
environment mutation. Native Unix bytes are retained. Descriptor APIs are Unix-
gated; non-Unix invocation/metadata builds were not cross-compiled in this slice.

## Dependency graph and local use

```text
bundle -> handles
       -> state -> handles
invocation -> std only
stdio      -> std only
handles    -> std only
facade     -> optional selected capability crates
local example -> facade(local) -> invocation + stdio
```

Facade defaults are empty. Individual features opt into matching crate reexports;
`local` groups only invocation and stdio. Enabling `bundle` includes its internal
state/handles dependencies but does not implicitly enable their facade reexports.
No placeholder remote features or empty runtime crates were added.

The [local example](/examples/local/src/main.rs) receives explicit context,
duplicates stdio safely, copies input bytes to output and writes invocation counts
to stderr. It uses no process-global mutation, command registry or subprocess
executor. Direct descriptor I/O assumes the caller coordinates any stdio buffering.

## Verification and baseline

Verified locally on x86_64-unknown-linux-gnu, rustc 1.101.0-nightly
`5c543b0b8` (2026-09-29), LLVM 23.1.1:

- `cargo nextest run --workspace --all-features`: 11 passing tests. Coverage:
  duplicate-owner retention and socket EOF, shared-file offsets, multiple bundles,
  shared role references, extension carriers, malformed/missing/extra inventory,
  range overflow, native non-UTF-8 values and real redirected stdin/out/err pipes.
  A subprocess validates captured non-UTF-8 environment/cwd without mutating test
  process globals. The child helper test also runs as a no-op in the normal suite.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace --all-features --doc`: passed; no doctests currently.
- `cargo check -p handle-off --no-default-features` and separately each feature
  `handles`, `state`, `bundle`, `invocation`, `stdio`, `local`: passed.
- `cargo tree -p handle-off --no-default-features`: facade alone.
- `cargo tree -p handle-off-local-example`: facade + invocation + stdio only.
- `cargo tree -p handle-off --no-default-features --features bundle`: bundle,
  state, handles only. Cargo.lock has only seven workspace packages, no registry
  packages. No Tokio, rkyv, systemd, libc crate or other external dependency.

Size method (default release optimization, no custom LTO/panic/size tuning):

```sh
cargo build --release -p handle-off-local-example --config 'profile.release.strip="symbols"'
stat --format='%n %s bytes' target/release/handle-off-local-example
ldd target/release/handle-off-local-example
```

Measured executable: **355,640 bytes**, dynamically linked to libc and libgcc_s
plus the loader. This is the whole example executable, not the incremental library
cost, installed footprint, memory usage or a static binary. No size target is
claimed. Remote comparison remains unavailable until a real transport exists.

## Acceptance accounting

- `handoff-foundation-inventory`: extraction map supplied above; toolkit vs
  application ownership grounded against actual deployed sources and retained
  legacy evidence.
- `handoff-foundation-contracts`: executable multi-bundle/extension/shared-
  reference contracts and explicit ownership checks implemented. Concrete wire-
  format/bootstrap, OS inspection and application compatibility remain later work.
- `handoff-foundation-workspace`: domain-grouped five-crate graph, optional
  facade and local dependency/size baseline implemented. The acceptance clause
  comparing a chosen remote build remains pending transport implementation.
- `handoff-cli-local`: explicit local handler, lossless native context and
  redirected byte I/O implemented; no remote/session/replacement claim.

Direction and remaining capabilities: [architecture](/design/fd-handoff/architecture0.gpt6a.md).
Recovered production precedent and limitations: [research1](/design/fd-handoff/research1.glm53max.md).
Workspace entry point and usage: [README](/README.md).
