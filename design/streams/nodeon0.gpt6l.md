---
type: Research
title: Nodeon patterns for enduring stream tools
description: Source-checked native verification APIs, selected design proposals, and concrete transfer guidance for source routing, file checkpoints, journal cursors and reproducible stream fixtures.
resource: /design/streams/nodeon0.gpt6l.md
status: research
generated: { by: "model:openai/gpt-6-astra#medium", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a2f3466ffej4ehYD59EjQJAS"
    title: Inspected Nodeon implementation and designs, existing handle-off URI work and systemd cursor documentation; derived stream-tool recommendations
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Existing resource, connection, credential and realization vocabulary
  - resource: /README.md
    title: Implemented optional handle-off primitives and current lifecycle limits
  - resource: "file:///home/rektide/src/nodeon/tool/verification/README.md"
    title: Current native scenario library, installed CLI and verification boundaries
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/scenario.rs"
    title: Actual definitions, workload context, materialization and retained attempts
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/catalog.rs"
    title: Actual caller-owned selection, sequential execution and partial reports
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/lib.rs"
    title: Actual verdicts, bounded reads, create-new writes and attached-service identity
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/process.rs"
    title: File-backed child capture and explicit process custody
  - resource: "file:///home/rektide/src/nodeon/tool/verification/examples/catalog.rs"
    title: Downstream borrowed-source composition example
  - resource: "file:///home/rektide/src/nodeon/tool/verification/tests/catalog.rs"
    title: Inspected selection, cancellation and retained finalization-failure tests
  - resource: "file:///home/rektide/src/nodeon/design/verification/README.md"
    title: Delivered catalog versus four directional refinements
  - resource: "file:///home/rektide/src/nodeon/design/verification/catalog0.gpt6a.md"
    title: Delivered catalog API and actual implementation deviations
  - resource: "file:///home/rektide/src/nodeon/design/verification/records0.gpt6a.md"
    title: Proposed bounded read-only packet navigation and report authority
  - resource: "file:///home/rektide/src/nodeon/design/verification/relationships0.gpt6a.md"
    title: Proposed run-scoped evidence links and concurrency-friendly attempt handles
  - resource: "file:///home/rektide/src/nodeon/design/verification/fixtures0.gpt6a.md"
    title: Proposed pure fixture assembly with provenance and collision rejection
  - resource: "file:///home/rektide/src/nodeon/design/verification/profiles0.gpt6a.md"
    title: Proposed explicit supported execution configurations
  - resource: "file:///home/rektide/src/nodeon/design/observability/synthesis0.gpt6a.md"
    title: Driver-first delivery, signal separation and earned toolkit extraction
  - resource: "file:///home/rektide/archive/systemd/systemd/man/journalctl.xml"
    title: Existing after-cursor and cursor-file behavior
  - resource: "file:///home/rektide/archive/systemd/systemd/man/sd_journal_seek_head.xml"
    title: Seek positioning, nearest-entry fallback and exact cursor checks
  - resource: "file:///home/rektide/archive/systemd/systemd/man/sd_journal_get_cursor.xml"
    title: Opaque journal cursors and semantic cursor comparison
verification:
  method: Read-only source and documentation inspection; explicit-file Markdown and OKF check
  limits: No Nodeon execution or durable-streams protocol qualification; proposed handle-off APIs and checkpoint semantics are recommendations.
---

# Nodeon patterns for enduring stream tools

**Borrow Nodeon's explicit library inputs, installed-binary fixtures and retained
first-attempt evidence. Build the stream reader and its file checkpoint as small
independent capabilities; Nodeon's runner is not already a generic stream harness.**

Authorship exception: the commission requested Luna xhigh and this exact filename;
the harness actually selected Astra medium. The filename is retained as requested,
but this is Astra-authored research, not a Luna run or an independent Luna judgment.

## Evidence boundary and reading map

Nodeon's inspected checkout was clean at Git commit
`88c7de78cd71636fafa2de9a88d650f6834a752c`. Local Nodeon links below are convenient
current-home links; that commit identifies the inspected source revision. No
project-local `AGENTS.md` was found in Nodeon or handle-off; supplied Rekon
instructions governed this work. Existing handle-off design was read before
proposing source vocabulary. Parallel implementation is outside this account.

Read these selected Nodeon documents in order:

| Document | Standing at inspection | Specific value here |
| --- | --- | --- |
| [Native driver guide](file:///home/rektide/src/nodeon/tool/verification/README.md) | Implemented | Runnable commands, public APIs, retained files, custody and exit semantics |
| [Caller-owned catalogs](file:///home/rektide/src/nodeon/design/verification/catalog0.gpt6a.md) | Delivered; original proposal retained below implementation account | In-memory declarations/selection separated from effects and report finalization |
| [Retained packets](file:///home/rektide/src/nodeon/design/verification/records0.gpt6a.md) | Proposal | Bounded offline reads, partial evidence, explicit authority, no implicit rerun |
| [Record relationships](file:///home/rektide/src/nodeon/design/verification/relationships0.gpt6a.md) | Proposal | Explicit multi-attempt evidence references; independent handles for overlapping reads |
| [Fixture bundles](file:///home/rektide/src/nodeon/design/verification/fixtures0.gpt6a.md) | Proposal | Assemble and validate bytes before writes; stable inventory with origins |
| [Execution profiles](file:///home/rektide/src/nodeon/design/verification/profiles0.gpt6a.md) | Proposal | Separate workload from concrete launch configuration; preserve requested versus observed settings |
| [Observation synthesis](file:///home/rektide/src/nodeon/design/observability/synthesis0.gpt6a.md) | Design synthesis; driver portion subsequently implemented | Deliver useful native tools before telemetry; distinguish verdicts, diagnostic events and spans |

The [refinement index](file:///home/rektide/src/nodeon/design/verification/README.md)
explicitly says only refinement 1 is delivered. Neither a generic packet query
API nor bundle/profile machinery should be described as available dependencies.
Nodeon's root README contains older product proposals; the maintained tool guide
and source are the stronger evidence for today's verifier.

## Actual reusable APIs and useful limits

### Catalog and scenario execution

In [catalog.rs](file:///home/rektide/src/nodeon/tool/verification/src/catalog.rs),
`Catalog::new(iter_of_definitions)` owns definitions while borrowing their bytes
and metadata. `snapshots()` lists declarations without filesystem effects or run
UUIDs. `select(&Selection { ids, tags })` rejects unknown IDs and empty matches;
matching is exact, OR within each group and AND between groups. Caller order is
preserved. Run tags annotate evidence, not selection or execution policy.

`Selected::run(&RunOptions { owner, client, packet, cancel, run_tags })` executes
sequentially. The caller supplies installed binaries and a nonexistent packet
path whose parent exists. Single-definition and catalog execution both own packet
creation/finalization. Cancellation records unstarted IDs rather than inventing
failed attempts. Child final-write failure retains the actual in-memory report
and UUID in the aggregate; independent later definitions can still run.

[scenario.rs](file:///home/rektide/src/nodeon/tool/verification/src/scenario.rs)
provides `SourceFile { path, contents }`, `Definition` with an async function-pointer
workload, and `scenario::run` for a caller-supplied closure. `Context::invoke`
retains request/outcome; `Context::command` retains an actual argv and raw output;
`Context::attempt` reserves a fresh evidence directory. Duplicate attempt IDs fail
before another call is sent. No global runtime, signal handlers or stdout policy
is installed by the library.

The [catalog example](file:///home/rektide/src/nodeon/tool/verification/examples/catalog.rs)
really borrows a local `String` as source bytes. It is a good usability acceptance
pattern: two caller-defined scenarios each get fresh mutable state through public
APIs, without central registration or `'static` fixtures. Embedded CLI discovery
uses a sorted build-time scan of `scenario.rs` files; installed runs do not need
the checkout. This is suitable for a finite stream-fixture catalog, not a dynamic
plugin system or a scheduler.

**Direct reuse limit:** `RunOptions` and `Context` are coupled to a Nodeon owner,
thin client, published binding, `Attached`, and CDP readiness. A generic HTTP stream
server or journal source cannot simply be substituted. The package excludes Deno
and V8 but still depends on Nodeon client/protocol and Tokio. Borrow the seam; do
not add a sibling absolute-path dependency or extract a general framework before
a second working consumer establishes its boundary.

### Process capture and durable evidence

[process.rs](file:///home/rektide/src/nodeon/tool/verification/src/process.rs)
offers `CommandSpec`, `OwnedProcess::spawn`, `run`, `finish`, and `ProcessRecord`.
It writes command metadata before spawning, redirects stdout/stderr directly to
fresh regular files, owns one `std::process::Child` reaper, and uses a dedicated
process group with bounded TERM/KILL cleanup. Its combined 64 MiB output cap is
polled and can overshoot. It does not establish hostile-descendant containment.
Its environment filtering is specifically Nodeon/OTel policy, not a universal
hermetic execution environment. Stdin is null; a pipe-fed stream fixture needs
an explicit input mechanism rather than assuming this runner already supplies it.

`Report` separates behavior, evidence and cleanup. A counterexample can remain
useful when capture is incomplete; successful cleanup cannot turn an empty
workload into a pass. Exact CLI bytes are separate from typed outcomes. The
current `report` command only bounded-reads `report.json`, not all artifacts.

Important distinction for checkpoint work: Nodeon's `write_json` uses
`create_new`, writes, and flushes. It is an immutable-evidence writer, **not** an
atomic-replacement or power-loss-durable checkpoint store: no rename or `sync_all`
appears there. Keep the checkpoint writer separate from retained-attempt files.

Existing usage worth trying separately, with installed binaries and fresh output:

```sh
nodeon-verify list
nodeon-verify fixture --nodeon /absolute/bin/nodeon --client /absolute/bin/nodeon-client \
  --scenario persistence --out /tmp/opencode/nodeon-stream-inspiration-fresh
nodeon-verify report --packet /tmp/opencode/nodeon-stream-inspiration-fresh
```

These commands are documented, not executed in this research. The guide reports
18 passing client/verifier tests at its catalog checkpoint; that is attributed
prior verification, not a fresh result here.

## Transfer to enduring CLI readers

The following is proposed handle-off behavior, not an existing Nodeon API or a
verified durable-streams client contract. Keep exact transport offset/framing
behavior with the chosen durable-streams implementation and its source review.

### Reuse the existing resource and URI vocabulary

The existing [remote-resource draft](../remote-resources/draft0.gpt6a.md) already
separates `id`, `kind`, `uri`, `connection`, credential references, representation
media types and local realizations. Extend that vocabulary for stream sources;
do not invent a competing URI registry or put credentials into checkpoint files.
The draft is itself a proposal, not evidence of an implemented catalog/resolver.

A useful routing sequence is: resolve a declared resource/ref with its explicit
base → validate the kind/connection context → choose an implemented adapter →
open the selected source → validate the checkpoint's binding. Scheme alone is
insufficient when several kinds use HTTPS. A local fixture endpoint and a remote
server sharing a path are different sources; resource ID alone is insufficient
if the record can be edited to point elsewhere.

Checkpoint binding should retain the effective source locator and relevant
non-secret connection identity, adapter/profile version, and source selection
(journal namespace/matches, for example). Preserve URI path/query semantics;
do not normalize remote keys as filesystem paths. Credential rotation need not
invalidate a checkpoint when the same logical source remains selected. A changed
source or filter should require explicit new state or a deliberate reset, not
silent reuse of the old cursor. Reuse the draft's relative-reference/base rules.

### One small file can hold committed progress

Illustrative fields, not a frozen schema:

```json
{
  "version": 1,
  "source": { "id": "events", "kind": "durable-stream", "uri": "https://example.invalid/events" },
  "selection": {},
  "position": { "scheme": "adapter-defined", "value": "opaque-token", "resume": "after" }
}
```

The adapter must define whether its token denotes the last committed record or
the next unread position. `after` here is illustrative for a last-record token;
do not apply it to a protocol's next-offset token. Avoid string arithmetic,
sorting opaque offsets, or deriving offsets from decoded/reformatted byte counts.

Start with single-writer state ownership, bounded versioned reads, and a sibling
temporary file followed by atomic rename. If claiming survival of power loss,
sync the temporary file and containing directory at the appropriate boundaries;
rename alone proves neither. Prevent concurrent writers from racing via an
explicit lock/lease or refuse shared state use. No database is needed for this
first capability.

Advance committed progress only after the declared output boundary succeeds:
for a basic CLI, complete record/batch write plus stdout flush. Partial writes,
broken pipes and cancelled output must not advance beyond complete committed
units. Keep any read-ahead cursor in memory separate from the persisted cursor.
If the transport exposes only a batch-end token, commit that token only after the
whole batch succeeds. A crash after output and before checkpoint replacement can
replay output. Stdout flush proves delivery to the OS, not downstream processing;
this is not exactly-once delivery. Stronger guarantees need an acknowledged or
transactional sink coordinated with progress, outside the simple CLI slice.

On checkpoint persistence failure, stop and expose the prior saved position and
uncertainty rather than continuing to emit unbounded uncheckpointed output.
Never replace malformed state with a default start position. Initial start policy,
explicit reset, reconnect backoff, follow versus finite read, and retention-gap
behavior should be visible CLI choices. Reconnecting a replayable read is useful;
Nodeon's no-replay rule for uncertain side-effecting calls should not be copied
as a blanket prohibition on stream reconnect. The output boundary determines the
possible duplicate range.

### Journal cursors have concrete native semantics

Nodeon supplies no journal reader precedent. The inspected systemd manuals add
the essential backend-specific facts:

- [`journalctl --cursor-file`](file:///home/rektide/archive/systemd/systemd/man/journalctl.xml)
  starts after a saved cursor and writes the last cursor at the end. It explicitly
  supports repeated sequential invocations. It is useful inspiration or a finite
  subprocess adapter, but not proof of continuous checkpoint updates in a follower.
- [`sd_journal_get_cursor`](file:///home/rektide/archive/systemd/systemd/man/sd_journal_get_cursor.xml)
  returns opaque text. Multiple strings can refer to the same entry; compare
  semantically with `sd_journal_test_cursor`, not parsed cursor components.
- [`sd_journal_seek_cursor`](file:///home/rektide/archive/systemd/systemd/man/sd_journal_seek_head.xml)
  can seek near a missing entry. Seek alone does not position a readable entry:
  move with `sd_journal_next`, then test the requested cursor. For an exact saved
  last-entry resume, advance once more after an exact match. Missing-cursor/nearest
  entry behavior needs explicit gap policy before claiming lossless continuation.

Bind progress to the selected journal source and match set. A timestamp is useful
diagnostic context, not a substitute cursor: realtime may be ambiguous and
monotonic timestamps require boot identity. For a journalctl adapter, capture raw
output and exit separately, retain `__CURSOR` where the chosen output format
provides it, and verify its exact framing/field behavior before adoption. Do not
let journalctl's cursor-file update become an unexamined downstream acknowledgment.

## A reproducible fixture workbench

Use ordinary Rust scenarios with source bytes and explicit executable inputs.
Retain a fresh packet per run with declaration, resolved source/configuration,
fixture input bytes, raw stdout/stderr, before/after checkpoint snapshots,
attempt outcomes and cleanup. Keep stable scenario IDs distinct from run IDs.
Do not overwrite a prior packet or recompute historical labels from today's catalog.

Borrow the *proposed* bundle assembler's smallest useful part: named byte bundles,
explicit prefixes, deterministic inventory, entrypoint/path validation and collision
rejection before materialization. Current Nodeon materialization validates/writes
one file at a time and can leave a partial tree; whole-inventory preflight is a
recommendation beyond that implementation. Record planned versus actually written
inputs if setup fails. A fixture path is not sufficient provenance for a binary:
retain version/content identity where available and resolved launch arguments.

Start with these concrete cases rather than a profile matrix or workflow language:

| Scenario | Decisive observation |
| --- | --- |
| Finite read then restart | Second real CLI starts from saved progress; combined bytes match declared records |
| Follow and orderly stop | New records appear; cancellation leaves a parseable checkpoint at the promised boundary |
| Read-ahead plus output failure | Checkpoint never passes fully emitted units, even when source read was further ahead |
| Crash after output/before save | Restart demonstrates the documented duplicate window rather than claiming exactly-once |
| Save obstruction | Writer failure is nonzero, prior checkpoint remains readable, no silent reset |
| Wrong source/filter | Reuse is rejected before output; an explicit fresh checkpoint succeeds |
| Missing journal cursor | Nearest-entry fallback is detected and the declared gap policy is exercised |
| Fragmented transport/batch | Split chunks and multi-record batches preserve exact bytes and correct commit boundaries |
| Disconnect/reconnect | Only committed progress is reused; request attempts and possible repeats remain visible |
| State corruption/version mismatch | Explicit diagnostic; source is not restarted implicitly |

A deterministic fake adapter should expose barriers at read/output/checkpoint
boundaries; avoid timing sleeps as the only control for crash-window tests. Add
one real installed durable-streams client/server pair and a real journal fixture
as separate integrations. A JSON recording of journal rows is a useful parser
fixture but does not prove native seek/rotation semantics. Keep explicit readiness
and owned teardown; attached services do not gain stop authority merely because
their PID appears in metadata.

The packet-reader design suggests a useful independent command: inspect a saved
run/checkpoint without source access, installed server binaries or any execution.
Use bounded metadata/artifact reads and explicit incomplete status. A missing
outcome is not an empty successful result. The relationship design suggests
linking a continuity assertion to both pre-restart and post-restart attempts;
matching their names is not evidence linkage. These are small useful conventions
even before a full query module exists.

## Recommended first landing and connections

1. Define explicit source routing inputs and opaque progress semantics against one
   chosen durable-streams adapter; align with the existing resource draft.
2. Implement the file checkpoint independently of rendering and transport, with
   source binding and a documented output/save crash window.
3. Expose finite and follow readers through a library seam; let the CLI own argv,
   signal handling, rendering and diagnostics. Keep stdout payload-only.
4. Drive the installed CLI through a small retained fixture library, exercising
   restart and failure boundaries as well as happy-path typed calls.
5. Add the journal adapter with exact cursor verification and explicit gap policy;
   reuse the checkpoint mechanics without forcing identical token semantics.

The [existing handle-off workspace](../../README.md) keeps carriers, codecs and
runtime policy optional. Stream checkpoints should follow that composition rule;
file-based reopening does not need to depend on systemd fdstore or change the
descriptor-constrained handle table. Resource descriptions connect this work to
the [remote-resource proposal](../remote-resources/draft0.gpt6a.md), while Nodeon's
[observation synthesis](file:///home/rektide/src/nodeon/design/observability/synthesis0.gpt6a.md)
explains why native verification should precede generalized telemetry/toolkit
extraction. If shared mechanics are later adopted, a library-side consumer account
would give Nodeon concrete feedback; this research makes no such adoption claim.

Only this research file is owned by the present task. Neighboring indexes and
implementation files are intentionally left to their current parallel owners.
