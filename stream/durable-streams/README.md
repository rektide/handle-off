---
type: ImplementationGuide
title: Durable Streams finite HTTP workbench
description: Pre-alpha CLI commands, source-bound atomic file checkpoints, survival boundaries and reproducible fixtures.
resource: /stream/durable-streams/README.md
status: pre-alpha
generated: { by: "model:openai/gpt-6.1-sol#high", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a2eba1effeABgmzwRldpSIfR"
    title: Astra initial implementation followed by explicitly selected Sol high hardening, deterministic tests, live-server verification and documentation
  - resource: /design/streams/durable-workbench0.gpt61s.md
    title: Protocol research, observed filesystem fixture and assessment
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Source locator versus identity and explicit connection context
  - resource: /design/streams/nodeon0.gpt6l.md
    title: Explicit inputs, installed CLI fixtures and retained attempts
  - resource: /design/streams/nodeon-review0.gpt6l.md
    title: Independent review recommending source/config binding and output/checkpoint failure tests
---

# Durable Streams workbench

**Pre-alpha: finite HTTP commands and enduring, independent file readers.**
Package `handle-off-stream-durable` supplies a library and `durable-workbench`
binary. It initially has an isolated workspace; root integration is parent-owned.
No upstream Rust client dependency, stream server dependency or handoff transport
dependency is required. The initial implementation was Astra medium; Sol high
subsequently hardened and verified it in the same session.

## Build and use

From the repository root:

```sh
cargo build --manifest-path stream/durable-streams/Cargo.toml
W=stream/durable-streams/target/debug/durable-workbench
URL=http://127.0.0.1:4437/v1/stream/demo
$W create "$URL"
printf 'first\n' | $W append "$URL"
$W head "$URL"
$W read "$URL" --state reader-a.json --incarnation demo-generation-1
printf 'second\n' | $W append "$URL"
$W read "$URL" --state reader-a.json --incarnation demo-generation-1
$W read "$URL" --state reader-b.json --incarnation demo-generation-1
$W status --state reader-a.json
```

Reader A's second read outputs only `second`; reader B starts from the beginning.
Each `read` performs **one** finite GET. It is not a daemon, follow loop, long-poll
or SSE client. Create/append/head print response headers as JSON; read emits the
raw HTTP response body on stdout and a summary on stderr. Append takes bounded
stdin. Both input and response default bounds are 1 MiB (`--max-bytes`).

Content type defaults to `application/octet-stream`; declare `--content-type`
consistently for create/append/read/reset when using another representation.
Media-type comparison ignores case and parameters, but checkpoint binding stores
the exact configured string. Binary responses concatenate message payloads;
HTTP does not expose file-log framing. JSON responses are arrays: each read is a
separate raw response, not a continuously valid JSON document or NDJSON stream.
Even an empty JSON read may output `[]`.

Forks are explicit server-side creations:

```sh
$W create http://127.0.0.1:4437/v1/stream/branch \
  --fork-from /v1/stream/demo
```

`--fork-offset` accepts an opaque server offset; omission selects the server's
current fork default. No automatic deletion, reset, pruning or reconciliation is
performed. Create/append disable HTTP retries and redirects; an ambiguous append
transport failure may already have committed server-side. There is no producer
idempotency/sequence interface yet.

## Checkpoint schema and source identity

```json
{
  "version": 1,
  "binding": {
    "source_url": "http://127.0.0.1:4437/v1/stream/demo",
    "incarnation": "demo-generation-1",
    "content_type": "application/octet-stream",
    "max_bytes": 1048576,
    "output_profile": "http-body-v1"
  },
  "next_offset": "opaque-server-offset"
}
```

The incarnation is **operator-declared**, not authenticated or server-verified.
The inspected server has no incarnation header; its ETag is offset-based, not a
generation token. Assign a new incarnation when a URL is deleted/recreated or
retargeted. Reusing the old declaration can silently attach to a different
stream. This CLI cannot detect that mistake.

URL parsing normalizes HTTP(S) URLs; queries, fragments and URL credentials are
rejected. Connections are direct: ambient proxy settings are disabled. There is
no authentication, configurable TLS trust, source registry or credential resolver.
Such connection settings must enter the binding if added later. Paths retain URL
semantics, not filesystem semantics. Offset strings are never parsed or counted;
the initial protocol sentinel is `-1`.

Existing checkpoints must match every binding field. Unknown schema fields,
versions and output profiles, malformed state and oversized state (>64 KiB) fail
closed. To deliberately rebind, change the response bound, or rewind:

```sh
$W reset "$URL" --state reader-a.json --incarnation demo-generation-2 --offset=-1
```

Reset replaces the file; it does not create a backup. `status` returns JSON `null`
for absent state and otherwise validates the checkpoint without contacting HTTP.
Both commands take the reader lock. State parent directories must already exist.

## Survival and output boundaries

1. Take a nonblocking advisory lock on `<state>.lock` for the operation.
2. Validate state/binding; fetch and buffer a complete, bounded successful response.
3. Validate content type/next offset and preflight checkpoint serialization.
4. Write all body bytes and flush the caller's output writer.
5. Write a same-directory temporary checkpoint, sync its file, atomically rename
   over the state path, then sync the containing directory.

Oversized/truncated responses and HTTP/protocol failures produce no output or
checkpoint advancement. Output write/flush failure leaves the checkpoint old,
but a partial output may already exist. **Output followed by a crash/save failure
can replay on restart.** A directory-sync failure can occur after rename, making
the reported save result uncertain; inspect state before retrying. Flushed stdout
does not prove that a pipe consumer persisted or processed anything. There is no
transaction with application effects and no exactly-once guarantee.

The sync/rename sequence targets local filesystems with normal POSIX semantics;
it has not undergone a machine power-loss test. State survives reader-process
replacement when files survive, but cannot restore deleted server bytes. Locks
are cooperative, not protection from malicious writers, renamed directories,
symlink aliases or deleting/replacing the lock sidecar. Use one stable trusted
state path per reader. The sidecar is retained deliberately. Keep output separate
from state/lock paths; never redirect stdout over either file.

## Library seam and fixtures

`HttpStream::{create, append, head, read}` handles HTTP independently of file
state. `Batch` contains bounded body bytes, opaque `next_offset`, and
`up_to_date`. `Binding::new` constructs normalized binding; `validate` checks
deserialized/publicly assembled values. `FileReader::{open, status, reset,
read_into}` owns lock and output-before-checkpoint ordering. APIs are synchronous
and return errors rather than installing runtimes or catching failure to print it.

```sh
cargo nextest run --manifest-path stream/durable-streams/Cargo.toml
cargo clippy --manifest-path stream/durable-streams/Cargo.toml --all-targets -- -D warnings
python3 stream/durable-streams/fixtures/live.py \
  --server /absolute/path/to/durable-streams-server \
  --client /absolute/path/to/durable-workbench \
  --scratch .test-agent/durable-streams
```

Eight deterministic tests cover HTTP commands, opaque offset escaping, independent
readers/resume, source/config mismatch, lock contention, malformed/unsupported
state, bounds, redirects, missing headers, truncation, failed write/flush and
save-after-output replay. The live fixture uses a real server binary, creates a
fresh retained attempt directory, records command/input/output files separately
from mutable checkpoints, kills/reaps the server before restart, and retains
store/metadata inspection. Failed runs retain partial attempts and a failure
report. It is a trusted local fixture, not a sandbox or general packet runner.

The [implementation report](/design/streams/durable-workbench0.gpt61s.md) records
the exact qualified server revision and observed fork/retention outcomes. The
[storage research](/design/streams/storage0.glm53m.md) describes other upstream
backends; its Caddy/TypeScript framing must not be confused with this Rust server's
little-endian `data.log` format.
