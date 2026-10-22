---
type: ImplementationReport
title: Durable Streams HTTP workbench and enduring file readers
description: Research, implementation boundaries and observed fixtures for a pre-alpha HTTP workbench.
resource: /design/streams/durable-workbench0.gpt61s.md
status: draft
generated: { by: "model:openai/gpt-6.1-sol#high", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a2eba1effeABgmzwRldpSIfR"
    title: Astra initial protocol research and implementation; explicitly selected Sol high hardening, fixture verification and final assessment
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Locator versus identity and explicit connection context
  - resource: /design/fd-handoff/naming-opportunities0.gpt6a.md
    title: Scoped names and resource continuity
  - resource: "https://github.com/thesampaton/durable-streams-rust/tree/f2db77f65f4d9d6a88b94a77b72cc9d678a2351a"
    title: Locally inspected server CLI, HTTP headers and read response implementation
  - resource: /design/streams/nodeon0.gpt6l.md
    title: Explicit installed binary inputs and retained attempts, distinct from atomic checkpoints
  - resource: /design/streams/nodeon-review0.gpt6l.md
    title: Independent review motivating source/config validation and output/checkpoint boundary tests
  - resource: /design/streams/storage0.glm53m.md
    title: Other upstream storage formats and separation of reader cursors from fork retention
---

# Durable Streams workbench

This report began before implementation. The harness initially selected Astra
medium despite the Sol-high commission: Astra wrote the initial research,
implementation and first live probe. The parent subsequently explicitly switched
this same session to Sol high. Sol hardened binding/checkpoint validation,
disabled retries/proxies, extended failure tests and retained attempts, and
completed verification/documentation. The filename is not an authorship claim.

## Research and intended boundary

A locator does not establish continuity. The inspected Rust server has no
incarnation header: its GET ETag is constructed from start/end offsets (and a
closed marker). It must not be used as a generation identifier. An explicit,
operator-maintained incarnation string will bind reader state to the intended
stream generation. Delete/recreate at the same URL requires a new value.

Use direct HTTP rather than the upstream client. The server exposes PUT create,
POST append, HEAD metadata and GET with `offset`; reads supply
`Stream-Next-Offset`. Offsets remain opaque strings, including the initial `-1`
sentinel. Forks use `Stream-Forked-From` and optional `Stream-Fork-Offset`.

A single finite read buffers a complete response up to a configured byte bound.
Oversized responses fail before output and checkpoint advancement. Output is
written and flushed before a temporary state file is synced, renamed and its
directory synced. A crash in between can replay output. This is not exactly-once
delivery or proof that a downstream process durably consumed stdout.

Reader state binds schema version, normalized HTTP URL, declared incarnation,
expected media type, response byte bound and output profile. A separate advisory
lock serializes processes sharing a checkpoint. Independent state paths are
independent readers. Status and explicit reset are useful inspection/recovery
operations; no background reconciliation or cleanup is planned.

## Implemented surface

Package `handle-off-stream-durable` lives at
[`stream/durable-streams`](/stream/durable-streams/README.md), initially as a
standalone nested Cargo workspace. The parent owns root integration. Dependencies
are anyhow 1, clap 4 with derive, fs2 0.4, reqwest >=0.12.28 with blocking/rustls
and defaults disabled, serde 1 with derive, serde_json 1, and tempfile 3.

`durable-workbench` exposes create/append/head/read/status/reset. The public
library separates HTTP `Batch` reads from the file reader. This is a useful finite
CLI rather than a scheduler, journal abstraction or general stream registry.
Configuration mismatches fail before HTTP; source incarnation remains an explicit
operator declaration. Retry and redirect policies are disabled. No credentials
or ambient proxies enter the initial direct-HTTP profile.

## Verification and actual observations

Built the reference server without modifying upstream:

```sh
cargo build --locked -p durable-streams-server --bin durable-streams-server \
  --target-dir /home/rektide/src/handle-off/.test-agent/durable-streams/server-target
```

Working directory was `/home/rektide/archive/thesampaton/durable-streams-rust`;
`git rev-parse HEAD` returned `f2db77f65f4d9d6a88b94a77b72cc9d678a2351a`.
Build succeeded with upstream manifest/deprecation warnings. Actual `--help`
confirmed `serve`, `list`, `export`, and `import`; import was not exercised.

Ran the maintained probe from handle-off:

```sh
python3 stream/durable-streams/fixtures/live.py \
  --server .test-agent/durable-streams/server-target/debug/durable-streams-server \
  --client stream/durable-streams/target/debug/durable-workbench \
  --scratch .test-agent/durable-streams
```

The initial probe passed at `.test-agent/durable-streams/probe-0p7zpcsn/`.
The hardened retained-attempt probe also passed at
`.test-agent/durable-streams/probe-22_umjfs/`; a final explicit build and probe
passed at `.test-agent/durable-streams/probe-te69bdk1/`. These are local scratch observations,
not portable evidence links or committed stores. Each rerun makes a fresh
directory; see `evidence.json`, `server.log`, `attempt-*/`, and exported JSON.

Observed, not merely inferred from source:

- A created binary root accepted `alpha\n`, then `beta\n`. Reader A resumed with
  only `beta\n`; independent reader B returned `alpha\nbeta\n`.
- A fork at the parent tail returned `alpha\nbeta\nchild\n` after its own append.
- Deleting the parent returned HTTP 204. After force-killing and restarting the
  server on the same filesystem store, a fresh fork reader still returned the
  complete inherited prefix and child suffix. The existing fork reader emitted
  no old bytes, then resumed with `after-restart\n` after another append.
- Offline `list --json` and `export --output ...` succeeded after server stop.
- The parent `data.log` was 19 bytes: LE-u32-length-prefixed payloads `alpha\n`
  and `beta\n`. Its `meta.json` had `state: Tombstone` and `ref_count: 1`.
  Child metadata named source `root` and fork offset
  `0000000000000002_000000000000000b`; child log contained its own payloads.
  This is the Rust file backend, not Caddy's big-endian `data.seg` format.

Eight deterministic nextest tests pass. They cover ordinary commands, URL/offset
encoding, two readers, binding mismatch, advisory lock, malformed/unsupported
state, oversized state/body, missing metadata, wrong media type, HTTP failure,
redirect rejection, truncated/chunked body, and write/flush/save failure. A
deterministic output writer moves the checkpoint directory during flush to make
the save fail: the old checkpoint survives under the moved directory, and retry
replays the already-emitted payload. This verifies the duplication boundary
without timing sleeps. Strict Clippy (`--all-targets -- -D warnings`) passes.
Both this report and the crate guide pass the Rekon document checker with `--okf`
(structure, link destinations, anchors and session provenance).

## Assessment and remaining boundaries

The first useful slice is implemented and qualified against a real Rust
filesystem server. It survives process replacement on retained files and does not
conflate fork references with reader retention grants. It does not qualify memory,
redb, Caddy or TypeScript backends, TTL/expiry, producer idempotency, corrupt-log
recovery, machine power loss, authentication or exactly-once downstream effects.
The server was killed after acknowledged operations, not during an append.

One complete response must fit the configured bound; the tool does not truncate
output and guess a partial offset. Changing that bound intentionally requires
reset/rebinding, preserving the offset explicitly if desired. The sync/rename
checkpoint sequence is distinct from immutable fixture evidence. A sync failure
after rename has uncertain persistence; inspect status rather than assuming the
old checkpoint. No automatic cleanup is authorized. Shared journal integration
can reuse the separation of source binding, opaque position and output boundary,
but does not yet have a common checkpoint trait in this crate.
