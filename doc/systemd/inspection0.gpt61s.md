---
type: ImplementationReport
title: Optional bounded systemd inspection and fdstore metadata membership
description: Read-only busctl adapter checkpoint, explicit service identity, observation limits, process bounds and attributed verification.
resource: /doc/systemd/inspection0.gpt61s.md
status: implemented
generated: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6b8650fffeM5meMdn7oL2CZs"
    title: Read adapter source/tests and facade integration; wrote this report and revised maintained exposure without broad check reruns
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented 420c5b76 and reported 87 workspace tests, pinned-nightly checks and read-only live manager snapshots
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: Independent review findings supplied by parent dispatch, including P2 pre-bound JSON allocation and cross-step incarnation/metadata limits
  - resource: /systemd/inspect/src/lib.rs
    title: Linux-only adapter contract and explicit metadata/command trust limits
  - resource: /systemd/inspect/src/inspector.rs
    title: Target, query limits, service identity bracketing, capabilities and receipt verification
  - resource: /systemd/inspect/src/data.rs
    title: Typed JSON/XML parsing, kernel identity, per-name membership and errors
  - resource: /systemd/inspect/src/runner.rs
    title: Aggregate deadline, combined output bound and dedicated process-group cleanup
  - resource: /systemd/inspect/tests/fixtures.rs
    title: Fake busctl commands, ownership/error/cleanup cases and optional read-only live test
  - resource: /systemd/inspect/Cargo.toml
    title: Isolated optional parser and OS dependencies
  - resource: /systemd/fdstore/src/lib.rs
    title: Unchanged borrowed receipts and NoManager verification bypass
  - resource: /handle-off/Cargo.toml
    title: Optional systemd-inspect feature independent of local and notification-only builds
  - resource: /handle-off/src/lib.rs
    title: Facade systemd_inspect reexport
  - resource: /doc/systemd/custody0.glm53max.md
    title: Primitive checkpoint whose separate verification seam this adapter implements
  - resource: /doc/systemd/activation0.glm53max.md
    title: Separate inherited-descriptor claiming boundary, contrasted with read-only inspection
  - resource: /rust-toolchain.toml
    title: Current pinned nightly-2026-09-30
---

# Optional bounded systemd inspection

Commit `420c5b76`, implemented by
`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`, adds
`handle-off-systemd-inspect` under `systemd/inspect` and the optional facade
feature `systemd-inspect` (reexport `handle_off::systemd_inspect`). Its public
adapter API is Linux-only. It fills the explicit verification seam in the
[custody primitives](/doc/systemd/custody0.glm53max.md), without adding an
inspector to notify/fdstore themselves or changing their receipt APIs.

**Inspection is observation, not transfer of execution permission.** No store,
remove, service launch, automatic pruning, supervisor or generation lifecycle
operation is performed. Barrier completion remains ordering, never acceptance.
Independent review found a P2 entry-bound parsing issue; its bounded-visitor
correction is pending as described below. No review acceptance or ticket closure
is claimed here.

## Explicit target and service incarnation

Construct `Target::new(canonical_service_name, expected_main_pid)` and
`Inspector::new(BusScope::{User, System}, target, limits)`. A nonzero expected
MainPID is required; aliases are not accepted as canonical identity. The PID
can be this process for self-custody or a caller-known external service PID.
Neither bus scope nor target is guessed from the environment.

`inspect()` uses `GetUnit`, not `LoadUnit`, and invokes busctl with activation
and interactive authorization disabled. Before collecting version, capacity
and store rows, it checks the canonical Unit `Id`, Service `MainPID` and a
nonzero 16-byte `InvocationID`. Afterwards it resolves the object path again
and repeats identity checks; an observed path/incarnation change fails rather
than returning a snapshot for an apparently different owner.

This is **bracketing, not an atomic transaction**: identity properties are
themselves serial reads, store contents can change between queries or after
return, and the API supplies no PIDFD identity proof or capacity reservation.
The caller supplies expected MainPID, not an expected InvocationID; the
InvocationID check detects changes across sampling, not a preselected earlier
incarnation. `verify_offer` returns the verification enum and discards the
snapshot identity; it does **not** pin one incarnation across offer, verification,
removal or later lifecycle steps. Do not treat a successful snapshot as permission
to resume I/O or retire originals.

## Offer membership and removal observations

`Inspector::verify_offer(&StoreReceipt)` calls the receipt's existing
`verify_with` hook. `NoManager` bypasses both fstat and external queries and
returns `Verification::NoManager`. Otherwise it retains the caller's borrowed
originals and compares their `KernelIdentity::from_fd` metadata with **every
dump row under the receipt's name**:

- Identity uses mode, device major/minor, inode, and rdev major/minor from
  `fstat`; process-local FD numbers are not compared.
- All expected unique metadata classes must occur. Missing rows, an unexpected
  identity under the same name, or duplicate indistinguishable observed rows
  fail closed (`Missing`, `Unexpected`, or `Ambiguous` in `Snapshot::membership`).
- Duplicate expected metadata classes collapse. Distinct objects under a
  common tag are supported; unrelated names do not invalidate that tag's match.
- `MatchedMetadata` maps to `Verification::Confirmed`; unsupported inspection
  maps to `Unsupported`; other membership outcomes, including disabled storage,
  map to `Rejected`. Query/parsing/identity failures remain errors.

**Confirmed means metadata membership only.** Independent opens of one inode
can match while carrying different offsets or open file descriptions. The
adapter cannot certify alias multiplicity, OFD sharing, historical offer
causality or revocation of other references. Access/open-status flags are not
compared: even opposite pipe ends may share the metadata used here. Paths and
flags are preserved for diagnosis, not identity proofs. Expected duplicates
collapse even when they represent independently opened descriptions. Counts or
names alone are never offer-confirmation evidence, and **Confirmed is not an
unrestricted proof that closing all originals is safe**. A lifecycle consumer
must retain these limits and establish its own resource/permission guarantees.

`verify_removal(&RemoveReceipt)` observes that no row carries the tag (or the
store is disabled) and returns `Confirmed`; a remaining row returns `Rejected`,
unsupported inspection returns `Unsupported`, and errors remain errors.
`NoManager` again bypasses queries. It performs no remove operation and proves
neither that the submitted removal caused absence nor that other named/unnamed
aliases have retired. A later store update can invalidate the observation.

## Capabilities, capacity and error distinctions

`Snapshot` contains `ServiceIdentity`, manager version, `ManagerReport` and
`StoreContents::{Available, Unsupported, Disabled}`. Available may be empty;
empty, unsupported and disabled are not interchangeable.

Store/remove/poll-control/barrier support uses documented version thresholds
219/236/246/246. An unparseable leading version leaves these `Unknown`;
vendor backports and access permissions are not negotiated by these thresholds.
Inspection support checks the actual introspected `DumpFileDescriptorStore`
output schema (`a(suuutuusu)`, upstream added in 254), not version alone.
Typed busctl JSON and nine-field rows are validated; malformed signatures,
types, names or XML do not become absent storage. XML has a node bound, does
not resolve the external introspection DTD, and rejects entity declarations.

`Capacity` exists only when both `FileDescriptorStoreMax` and
`NFileDescriptorStore` are available with the expected property types. Missing
properties mean unknown capacity, not zero. Occupancy inconsistent with an
available dump fails with `InconsistentInventory`. Capacity arithmetic is a
snapshot, not a reservation, and matching membership can succeed with unknown
capacity.

Permission denial, unsupported members, disabled store, arbitrary command
failure, missing executable/I/O, timeout, output limit, malformed JSON/XML,
wrong unit/PID and changing identity stay distinct. Successful replies use
typed machine-readable output; busctl failures are unstructured text. The
runner forces the child's C locale, classifies recognized denial/unsupported/
disabled messages, and leaves other failures as `CommandFailed` rather than
guessing that storage is absent.

## Process and dependency boundary

`QueryLimits` defaults to **3 seconds for the aggregate serial query sequence**,
**1 MiB combined stdout/stderr per command**, and **16,384 dump entries**.
Output and entry ceilings are configurable up to 8 MiB and 65,536 respectively;
time must be positive and representable. These are not exact allocator-memory
limits or hard-real-time guarantees: parsing, scheduling and process cleanup
also take work. At this baseline the entry limit is checked **after** building
a full JSON `Value` and cloning its `data`; it does not yet bound JSON row
materialization. The output-byte bound still applies. See the pending review
correction below rather than treating the entry bound as an early allocation
guard.

Each busctl runs without shell interpolation in a dedicated process group,
with nonblocking output pipes and no reader threads. Timeout/output/I/O failures
before completion trigger group SIGKILL, direct-child kill and reap. The trusted
executable must not daemonize or escape that group; malicious escaped descendants
and unkillable kernel tasks are outside the contract. `with_program` permits an
explicit trusted executable override; the default resolves `busctl` through PATH.
Locale/color settings affect the child only, not process-global environment.

The adapter adds Linux-only `serde_json` and `roxmltree`, reuses rustix
(`std`, `fs`, `event`, `process`), and depends on the workspace fdstore crate.
It requires external busctl at query time, but no Rust D-Bus library or async
runtime. Empty defaults, `local`, activation and notify/fdstore-only builds do
not select it. No live manager is needed merely to build the primitives.

## Verification and attribution

Reported by implementer `opencode://ses_ec717edaeffeP56i2EfWqe8r3v`, **not rerun
by this documentation session**:

- **87 workspace tests passed**, with one optional live test skipped normally;
  clippy, formatting, doctests, strict rustdoc and feature checks passed on
  the current pinned `nightly-2026-09-30`.
- Separate read-only live snapshots passed for `dbus-broker.service` and
  `opencode-bus-proxy.service` on user manager **systemd 262-1**. These queries
  made no store/remove mutations and did not exercise live offer acceptance.
- Fixture tests compare real borrowed originals with fake busctl inventory,
  cover wrong/missing/ambiguous/unexpected identities, unsupported/disabled/
  unknown-capacity distinctions, permissions and malformed replies, changed
  incarnation, explicit scopes, observed removal, and timeout/output cleanup
  including a same-group descendant. A real-file test demonstrates equal inode
  metadata with independently positioned open file descriptions.

This session read source, tests, manifests and facade wiring, compared the
documented limits/outcomes with those paths, and checked documentation links
and provenance. It performed no broad test reruns or live manager commands.

## Review finding and pending correction

Reviewer `opencode://ses_ec6f1618effeapZzCE6xoMvdTD`, via parent dispatch,
identified **P2: JSON Value parsing and data cloning precede max_entries
enforcement** in `420c5b76`. A separate implementation session
(`opencode://ses_ec6b80ea4ffei4EU41MCJuKm5t`) is correcting this with a bounded
visitor; this report has not received a resulting commit or verification and
does not claim it fixed. The baseline checks above remain attributed baseline
checks, not correction checks.

The reviewer reported no other direct low-level custody-verifier blocker, while
emphasizing MainPID-only target pinning, per-call rather than cross-step
InvocationID bracketing, and metadata equality's access/OFD/alias limitations.
These are explicit caller/lifecycle constraints, not strengthened guarantees
conferred by the test count or read-only live snapshots.

## Cross-references

- ⇄ [Custody checkpoint](/doc/systemd/custody0.glm53max.md#later-inspection-checkpoint):
  retained primitive guarantees and the later optional inspection seam.
- [README — manager custody](/README.md#manager-custody): maintained feature
  exposure and the short observation boundary.
- [Socket activation](/doc/systemd/activation0.glm53max.md): inherited-FD claiming
  is a separate ownership operation, not a result of these snapshots.
