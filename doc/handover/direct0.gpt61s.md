---
type: ImplementationReport
title: Direct dormant-successor control
description: Separate correlated Unix control, core permission integration, retained failures and attributed real-process continuation evidence.
resource: /doc/handover/direct0.gpt61s.md
status: implemented
generated: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec62244e1ffeTltEwkpyaPtBmE"
    title: Read implementation, process fixture and design; documented actual APIs and evidence limits without rerunning checks
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented 4fb48b40 and f16957ff; reported 116 passing tests, 11 new and pinned-nightly checks via parent dispatch
  - resource: "opencode://ses_ec7f40693ffew22J16HkhJkhFP"
    title: Selected separate direct control architecture, commissioned fresh documentation and accepted the review's explicit unwind boundary
  - resource: "opencode://ses_ec622add3ffeZPbzPPOLe2WBfe"
    title: Fresh source-only review via parent dispatch found no execution-authority safety blockers and identified retirement-unwind ownership limits
  - resource: /handover/direct/src/lib.rs
    title: Cooperative control contract and wire-v1 frame
  - resource: /handover/direct/src/unix.rs
    title: Correlation, sender/receiver APIs, retained failures and nine unit tests
  - resource: /transport/unix/src/unix.rs
    title: Read-only offer correlation and unchanged custody-only delivery
  - resource: /examples/direct-handover/src/main.rs
    title: Successor dormant pending-write restore, release and retirement
  - resource: /examples/direct-handover/tests/processes.rs
    title: Parent-side core integration and two real-process stream continuation tests
  - resource: /handover/direct/Cargo.toml
    title: Workspace dependencies with no mandatory runtime, memfd or systemd
  - resource: /handle-off/Cargo.toml
    title: Optional handover-direct feature outside local and empty defaults
  - resource: /design/fd-handoff/lifecycle0.gpt6a.md
    title: Separate control channel, application-owned parked state and irreversible relinquishment direction
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E4.2 multiple-bundle acceptance direction and limits beyond this fixture
  - resource: /design/fd-handoff/work.md
    title: Native work ownership and requested fresh-agent rollover
  - resource: /doc/handover/core0.gpt61s.md
    title: Long-lived controller, trusted evidence and one-shot authority approvals
---

# Direct dormant-successor control

Commit `4fb48b40` adds read-only bundle offer correlation; `f16957ff` adds
`handle-off-handover-direct`, facade feature/reexport `handover-direct` /
`handle_off::handover_direct`, and a real-process pending-write example.
Implementation is by `opencode://ses_ec717edaeffeP56i2EfWqe8r3v`.
This report documents the checkpoint, not ticket acceptance. Fresh source-only
review via parent dispatch (`opencode://ses_ec622add3ffeZPbzPPOLe2WBfe`) reported
no execution-authority safety blockers and confirmed the same-live-stream process
evidence, with the retirement-unwind ownership limit described below. The parent
accepted that explicit boundary for this checkpoint, not an API redesign.

## Three different facts

1. **Bundle custody:** transport `Offer::accept` sends its ACK and yields a
   `Delivery`. It does not grant execution or prove application reconstruction.
2. **Dormant readiness:** receiver validates/restores sufficient application state
   and sends READY. Sender receives generation/offer-correlated readiness as
   trusted evidence for core `verify_custody`, not permission for either owner I/O.
3. **Execution release:** the application permanently relinquishes old execution;
   core then issues its one-shot `ReleaseGrant`. Direct RELEASE carries the
   decision to the successor, which receives a non-Clone `ReleaseNotice`.

The [bundle transport](/doc/transport/bundles0.glm53max.md) stays custody-only.
The [core](/doc/handover/core0.gpt61s.md) stays backend-free. This adapter neither
parks an application nor launches/replaces a process nor owns all restored state.

## Connection and correlation contract

Both peers supply a fresh already-connected AF_UNIX SOCK_DGRAM control socket,
**separate from the bundle socket**, with exclusive use and initially empty
queues. `Sender::new` / `Receiver::new` take a `Correlation` and finite nonzero
`Duration`, verify a connected peer and configure control read/write timeouts.
Bundle transport timeouts remain independently caller-configured.

`Correlation::new(session, generation, offer_id)` requires a nonzero offer ID
and generation of at most 256 UTF-8 bytes. The application supplies a fresh
128-bit session token; no randomness/authentication provider is installed.
Sender can retain the planned correlation **before publication**, using
`Endpoint::next_offer_id()`. Receiver reads `Offer::offer_id()` before accepting.
These accessors do not reserve an offer, alter framing or strengthen a receipt.
Serialize endpoint use so the planned offer remains the submitted one.

All control frames must match session, generation and offer ID. Wire version 1:

| Bytes | Meaning |
| --- | --- |
| 0–7 | `HODIR01\0` magic |
| 8–11 | little-endian u16 version, u8 kind, zero u8 |
| 12–27 | session token |
| 28–35 | little-endian u64 offer ID |
| 36–39 | little-endian u16 generation length, zero u16 |
| 40 onward | exact UTF-8 generation bytes, at most 256 |

Kinds are READY=1, RELEASE=2, RELEASED=3, ABORT=4, RETIRED=5. There is no body
or ancillary FD inventory. Receive bounds are 296 bytes / zero FDs; unexpected
rights are rejected and closed by the underlying rights primitive. Unsupported
versions, malformed/unrelated frames and unexpected kinds fail closed rather
than being skipped in a receive loop.

## Actual APIs and core integration

| API | Meaning |
| --- | --- |
| `Receiver::stage(offer)` | Checks generation/offer correlation and accepts transport custody. Returns `StagedDelivery`; no restore or READY yet. |
| `Receiver::ready(&staged, validate_restore)` | Trusted callback validates/restores dormant application state. Only success sends READY. Application owns partial restored state/copies. |
| `Sender::wait_ready(&Receipt)` | Checks the transport receipt against correlation, then waits for READY. Returns `ReadyReceipt`. |
| `Sender::release(ReleaseGrant)` | Requires matching generation and observed readiness; sends RELEASE and waits for RELEASED. |
| `Receiver::wait_decision()` | One terminal receive yields `Decision::Release` only after readiness, or `Decision::Abort` even before readiness. |
| `Receiver::activate(staged, notice)` | Returns delivery plus received permission after sending RELEASED. Does not execute application I/O. |
| `Sender::abort()` | Sends ABORT and waits for RETIRED. Allows one preceding READY when abort crosses initial readiness after lost bundle ACK. |
| `Receiver::retire(staged, notice, retire_app)` | Callback retires all app copies/actors/restoration paths; then library delivery is dropped **before** RETIRED is sent. |

The sender application retains originals, parked state, the possible publication
plan and control session alongside a long-lived core `ParkingController`:

1. Core `publish` wraps `Endpoint::send`; keep the resulting custody receipt.
2. Core `verify_custody` wraps `Sender::wait_ready`, retaining correlated evidence.
3. Core `relinquish` must return an honest `RelinquishOutcome`: only permanent
   relinquishment allows subsequent release; ambiguity never means old resume.
4. Core `release` passes its grant to `Sender::release`. RELEASED is permission
   receipt, **not application completion**; successor may fail afterward.

Before relinquishment, core `abort` can wrap `Sender::abort`. Only successful
complete retirement evidence permits core to issue a matching old `ResumePermit`.
The direct adapter never issues that permit itself. After relinquishment or lost
release acknowledgment, this old-resume path is forbidden.

## Retention and uncertainty

Control operations poison the session before network work, and clear poison only
on success; terminal decisions spend it. Timeouts/invalid replies after entry
remain conservative uncertainty. There is no automatic retransmission, second
execution approval, old-owner rollback, path cleanup or application resume on Drop.
Preflight errors such as a mismatched receipt need not poison an untouched channel.
Finite socket-operation deadlines do not bound application callbacks or the whole
handover; applications must bound their own validation/retirement work.

- `StageFailure.retained` is `RetainedStage::Unaccepted(Offer)` for preflight
  rejection, or `Accepted(StagedDelivery)` when the transport ACK failed after
  custody was acquired. An ACK failure does not erase the received inventory.
- `ReadyError::Application` sends no READY; the borrowed staged inventory and
  any partial app reconstruction remain caller-owned. `ReadyError::Control`
  does not undo reconstruction and may mean READY reached the sender.
- `ReleaseFailure` retains the original core grant. Retention is recovery data,
  not a renewed grant or permission to resume old execution on a poisoned session.
- `ActivationFailure` retains staged delivery **and received release notice**:
  a failed RELEASED send does not retract permission already received.
- `RetirementFailure` retains delivery (`Some`) on preflight/app retirement error.
  If app retirement succeeded and RETIRED send fails, library custody is already
  dropped (`None`); sender still cannot assume retirement from a missing reply.

Application parked state remains outside these error types. Retain actual owners
and evidence deliberately; merely dropping a returned error can close its owned
FDs. Core attempt Drop quarantines bookkeeping, not a backend reconciliation job.

**Panic boundary:** `Receiver::retire` takes `StagedDelivery` and `AbortNotice`
by value before invoking `retire_app`. If that callback unwinds, these owners and
the notice drop rather than being returned in `RetirementFailure`. No RETIRED
acknowledgment is sent, so the old owner remains parked; this is not a dual-owner
execution grant, but it is **not ownership retention across panic**. Application
copies remain subject to their own unwind/lifetime policy. Existing retirement
tests exercise returned errors, not callback unwind; no panic test is claimed.

## Real-process example and evidence

The [successor binary](/examples/direct-handover/src/main.rs) takes four explicit
data/control local/peer paths. It restores one `example.pending-write` v1 inline
image: little-endian u64 offset followed by pending bytes, capped at 4096 image
bytes. It rejects unsupported extension scopes/shape, clones the transferred
stream and queries `peer_addr` while dormant. It performs no stream I/O or shared
timeout/flag mutation until permission. Abort explicitly drops the application
clone before the library inventory is retired.

The [process fixture](/examples/direct-handover/tests/processes.rs) supplies the
old-owner driver and launches this binary; the binary is not a standalone sender
or supervisor. To reproduce the two tests on the pinned toolchain:

```sh
cargo nextest run -p handle-off-direct-handover-example
```

The parent writes `abc` of `abcdefghij\n` to a live Unix stream, transfers that
same stream with pending offset 3, waits for READY, and asserts no new bytes during
a bounded dormant interval. On release the child emits exactly `defghij\n`, then
answers ping/pong on the same connection. In the abort variant, retirement is
acknowledged before core permits the old writer to emit the suffix. Old FD
ownership survives release: no OS revocation is claimed; the fixture enforces
its cooperative old-writer gate by code discipline.

This is two real processes and real transferred stream references, not merely
synthetic gates. It is **one resource / one bundle**, not the architecture's full
multiple-bundle E4.2 fixture. Existing multiple-bundle transport evidence is
separate; it does not prove multiple-bundle execution coordination here. There
is no CLI framework/result channel, manager generation publication/retirement,
live-systemd replacement or deployed bus-proxy change in this checkpoint.

Via parent dispatch, the implementer reports **116 passing workspace tests,
11 new** (nine adapter unit tests plus these two process tests), with one optional
live test skipped, and passing pinned-nightly clippy, fmt, strict rustdoc and
feature checks. This documentation session read source/manifests/design and
checked links; it did **not rerun** implementation tests or those build checks.
After adding the retirement-unwind rustdoc note, this session separately ran
`RUSTDOCFLAGS='-D warnings' cargo doc -p handle-off-handover-direct --no-deps`
successfully on the repository-pinned toolchain. This is a targeted documentation
check, not a workspace verification rerun.
Unit coverage includes wrong/stale correlation, malformed/versioned frames,
unexpected control FDs, timeouts, duplicate release, crossed READY/abort, lost
release/retirement ACKs and retained partial restore/activation/retirement failures.

## Trust and dependency limits

Peer trust and endpoint permissions are application policy. Session correlation
is not authentication; public receipt fields and core evidence are attestations,
not unforgeable OS evidence. Both peers must prevent conflicting I/O, aliases,
shared offset/flag changes, auto-resuming leases and controller resets. Dormant
restore must avoid seeking, O_NONBLOCK changes, shutdown or protocol writes.
Retirement success must cover **all** copies and actors, not just `Delivery`.
Generation/domain redemption and truthful irreversible relinquishment remain
application duties. No arbitrary crash recovery or exactly-once execution follows.

The crate directly depends only on core handover, Unix bundle transport and Unix
rights workspace crates; selected transport transitively brings rkyv/rustix.
No mandatory async runtime, memfd, systemd/inspector or invocation dependency is
added. Facade `handover-direct` stays outside `local` and empty defaults: local
invocation/stdio consumers retain their isolated dependency graph.

## Cross-references

- [README — direct successor control](/README.md#direct-successor-control):
  maintained entry point to this report and its process evidence.
- [Lifecycle direction](/design/fd-handoff/lifecycle0.gpt6a.md#direct-adapter-decision):
  separate control decision and trusted-copy retirement obligation implemented here.
- [Core checkpoint](/doc/handover/core0.gpt61s.md): authority/quarantine contract;
  its deferred-direct wording describes the earlier core-only checkpoint.
- [Active work map](/design/fd-handoff/work.md): parent-owned tracker/landing history;
  direct review and acceptance updates belong to that parent, not this report.
