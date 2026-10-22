---
type: Research
title: Published durable-streams crate as an alternative server fixture
description: Package-verified source inspection of durable-streams 0.1.5, its file ranges and tiered segment manifest, and a bounded second-fixture proposal.
resource: /design/streams/crate-fixture0.gpt6a.md
status: research
generated: { by: "model:openai/gpt-6-astra#medium", at: "2026-10-22" }
sources:
  - resource: "opencode://ses_e9afd800bffeVHLHIvV9eCKpzQ"
    title: User commissions an alternative crate fixture; parent verifies package identity and inspects storage and manifests
  - resource: "https://crates.io/api/v1/crates/durable-streams"
    title: Registry version, binary-only package, checksum and publication provenance
  - resource: "https://crates.io/api/v1/crates/durable-streams/0.1.5/readme"
    title: Published server commands, configurations, durability and conformance claims
  - resource: "https://docs.rs/crate/durable-streams/0.1.5/source/src/store.rs"
    title: Inspected packaged StreamState, file-range Segment, sidecar Meta and MetaSegment definitions
  - resource: "https://docs.rs/crate/durable-streams/0.1.5/source/src/tier.rs"
    title: Inspected packaged sealed-segment Placement, SegmentEntry and Manifest definitions
  - resource: /design/streams/storage0.glm53m.md
    title: Earlier source account of Caddy and TypeScript backends, distinct from this implementation
---

# A second Rust server fixture

**The published `durable-streams` crate is a separate server implementation. Its
optional tiering code has actual sealed segments and a manifest.** This is closer
to the original named-segment direction than the Caddy single-file backend.
It is useful as a second HTTP fixture and source precedent; no adoption of its
storage engine is selected here.

Work anchor: `handoff-streams-crate-fixture`.

## Exact artifact and evidence

On 2026-10-22 the registry reported latest non-yanked version **0.1.5**. The archive
was downloaded and verified against SHA-256:

```text
472721e61ca191520c5e2b9bf8859aa8f9aa85599027f9103addbf695724b8ad
```

The packaged `.cargo_vcs_info.json` reports revision
`88793e76595d69be300731b9b25c58538923a53b`, `dirty: true`, and path
`packages/durable-streams-rust`. Registry trusted-publication metadata names
`electric-sql/electric` at that revision, although the package repository field
points at `durable-streams/durable-streams`. The latter checkout at `461b402`
contained only `packages/client-rust/Cargo.toml` when searched. Consequently the
**published archive and its checksum** are the evidence binding here; a checkout
at the reported revision is not asserted to reproduce a dirty publication.

Scratch extraction is under `.test-agent/stream-crate/`, with an explanatory
README. No package code, builds, conformance tests or benchmarks were executed.
The findings below distinguish source structures from README operational claims.

## What can be run or linked

The normalized Cargo manifest declares one binary, `durable-streams-server`, and
no library target. Defaults have no optional features; `tier` enables the
`object_store` S3 adapter and `telemetry` enables OpenTelemetry dependencies.
The package declares edition 2021 and Rust 1.75; neither build compatibility nor
that MSRV has been checked here.

Its README describes direct flags rather than the thesampaton server's `serve`
subcommand:

```sh
durable-streams-server --host 127.0.0.1 --port 4438 --data-dir ./fixture-data
```

Both implementations install a binary with this name. Future fixtures must take
an explicit executable path and retain backend identity/version and exact argv;
PATH lookup alone cannot identify which server ran. This is a process/HTTP
integration candidate, not an available Rust library dependency.

## Actual storage concepts

### Live backing and read ranges

`store.rs` defines `StreamState` with an open-file-backed live stream, its logical
`base_offset`, optional parent `Arc<StreamState>`, appender mutex, shared state,
wake channel, metadata lock, tail cache and optional tier state. A fork's logical
base and the live file's physical base can differ after compaction.

Its `Segment` is a **read range**, not necessarily a separately sealed file:

```text
Segment { file: Arc<File>, file_start: u64, len: u64 }
```

The inspected `materialize_segments` uses positional `read_exact_at`, so reading
does not consume a shared file seek position. That helper returns empty bytes on
any read failure; it should not be copied into a reader that needs to distinguish
I/O failure from an empty event stream.

The README describes contiguous wire-byte storage rather than four-byte
per-message framing. For JSON, `tier.rs` explicitly describes the stored form
as `value1,value2,value3,`; sealing scans for safe top-level value boundaries.
Do not apply the Caddy or thesampaton record framing to these files.

### Sealed segments and the manifest

The source distinguishes the read-range `Segment` from a sealed `SegmentEntry`:

```text
SegmentEntry {
  logical_start,
  len,
  placement: Local(path) | Remote(key),
  remote
}

Manifest {
  segments: ordered entries,
  sealed_offset,
  offloading,
  deleted
}
```

Entries describe contiguous logical ranges starting at the stream's base.
Earlier fork-prefix data resolves through ancestry. `sealed_offset` separates
sealed history from the growing live file. Tiering is inactive by default.

`store.rs` includes a local filesystem blob-store adapter, selected by
`TierKind::Local`, as well as feature-gated S3. The source comments describe
`--tier local`; the published README's main flag table lists only `off | s3`.
Verify actual CLI acceptance before writing a local-tier fixture recipe.

This offers a useful precedent for a name resolving to a logical range and a
current backing placement. It does not implement our FD grants or a memfd
carrier. A source-local `Arc<File>` is not an interprocess handoff protocol.

### Persisted metadata and durability

`Meta` sidecars include stream ID/path/content type; TTL/expiry and last access;
closure and producer state; parent/fork boundary/base; fork reference count and
soft deletion; segment entries and sealed watermark; file base and pending
compaction intent; and an optional durable-tail frontier.

Persisted `MetaSegment` contains logical start, length, optional remote key and
optional local filename. Runtime offload flags are not themselves a transferable
manifest schema. The code comment says producer/access updates are debounced
without fsync while create/close/delete metadata writes are synchronous.

The README describes a sharded WAL with header/payload CRC32C and group-commit
durability, plus a `memory` mode that skips WAL/fsync. That mode still writes
buffered files: **it is not a memfd backend**. Its crash guarantees and advertised
conformance results are upstream claims, not results established by this account.

## Proposed second-fixture experiment

1. Reuse our HTTP reader driver with an explicitly supplied alternative server
   binary and a fresh retained data directory.
2. Repeat create, append, independent-reader checkpoint and restart cases against
   this server, treating offsets as opaque. Record differences rather than making
   its offset shape a shared test assumption.
3. Verify local-tier CLI support, then use a small segment threshold to produce a
   real manifest and sealed history. Inspect sidecars and compare logical range
   reads before and after a seal/compaction.
4. Keep this fixture optional until those checks pass. It can become the useful
   segment/placement experiment without displacing the initial thesampaton fixture.

Current compatibility finding: both claim Durable Streams support, but no exact
protocol-revision or conformance equivalence was established for this archive.
HTTP operations and configurations must be qualified by the fixture itself.

## Connections

- [Earlier storage account](storage0.glm53m.md): comparison evidence, not a
  universal statement that all Durable Streams implementations use `data.seg`.
- [Experimental tools](/stream/README.md): independent source readers and explicit
  checkpoint semantics are the reusable interface to both server fixtures.
- [Resource records](/design/remote-resources/draft0.gpt6a.md): placement belongs
  with locator/context; logical range identity need not change when backing moves.
