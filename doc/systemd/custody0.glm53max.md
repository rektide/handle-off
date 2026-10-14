---
type: ImplementationReport
title: Service notifications, ordering barriers and fdstore offers
description: Checkpoint record of the Sol-implemented E3.2 slice covering the runtime-free notify primitive with explicit NoManager/Submitted submission and bounded pipe barriers, the borrowed-receipt fdstore offer/remove/verify layer, the rendezvous facade integration, and four strict-rustdoc doc-comment repairs.
resource: /doc/systemd/custody0.glm53max.md
status: implemented
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
revised: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6b8650fffeM5meMdn7oL2CZs"
    title: Preserved the prior report; completed the 4096-byte correction and linked inspection delivery and bounded-parser correction evidence
  - resource: /doc/systemd/inspection0.gpt61s.md
    title: Later optional adapter fills the inspection seam without changing primitive submission or barrier guarantees
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: Independent review identified the systemd receive-buffer mismatch and required exact-boundary regression coverage
  - resource: file:///home/rektide/archive/systemd/systemd/src/basic/constants.h
    title: NOTIFY_BUFFER_MAX is PIPE_BUF (line 35)
  - resource: file:///home/rektide/archive/systemd/systemd/src/shared/notify-recv.c
    title: Receive buffer and truncated-payload rejection (lines 96–100 and 120–130)
  - resource: "opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB"
    title: Documentation checkpoint verifying the custody slice, repairing four rustdoc links, and extending the README exposure
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented notify and fdstore in 29271d39, integrated rendezvous behind facade features in a4dfb8f2, and reported the workspace checks recorded below
  - resource: "opencode://ses_ec6d652c3ffespR5CgFSQ5uwz6"
    title: Authored the rendezvous crates and their implementation report, read for integration context
  - resource: /systemd/notify/src/lib.rs
    title: Notify contract, submission semantics, message validation and bounded barrier loop
  - resource: /systemd/notify/tests/manager.rs
    title: Fake-manager ordering, barrier-timeout, malformed-input, queue-exhaustion and credential tests
  - resource: /systemd/notify/Cargo.toml
    title: Unix-gated rights and rustix std/event/pipe dependencies
  - resource: /systemd/fdstore/src/lib.rs
    title: FdName rules, capability/capacity data, borrowed receipts and verify_with hooks
  - resource: /systemd/fdstore/tests/manager.rs
    title: Named offers, no-manager behavior, barrier-not-acceptance and abstract-endpoint tests
  - resource: /systemd/fdstore/Cargo.toml
    title: Notify dependency with rights as dev-only
  - resource: /doc/rendezvous/implementation0.glm53max.md
    title: Rendezvous contract and local-policy report by the implementing session
  - resource: /Cargo.toml
    title: Workspace membership for both custody crates and the rendezvous pair
  - resource: /handle-off/Cargo.toml
    title: Separate facade features isolated from local
  - resource: /handle-off/src/lib.rs
    title: Facade reexports and updated crate docs
  - resource: /Cargo.lock
    title: Workspace-only lock growth for this slice
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E3.2 acceptance criteria
  - resource: /design/fd-handoff/work.md
    title: Checkpoint coordination and toolchain policy
  - resource: /rust-toolchain.toml
    title: Repo-pinned mise-selected nightly-2026-09-30
  - resource: /README.md
    title: Maintained exposure extended by this checkpoint
  - resource: /rendezvous/local/src/lib.rs
    title: Doc-comment link repair (cross-crate intra-doc paths)
  - resource: /invocation/core/src/lib.rs
    title: Doc-comment bracket-escape repair
  - resource: /transport/unix/src/lib.rs
    title: Doc-comment bracket-escape repair
  - resource: "opencode://ses_ec6088918ffeIq7D1VJbVIsyA5"
    title: "Ticket prefix rework to handoff IDs; bus-proxy ticket IDs normalized to busprox"
---

# Service notifications, ordering barriers and fdstore offers

`handle-off-systemd-notify` and `handle-off-systemd-fdstore`, committed in
`29271d39` by the Sol medium session
(`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`), deliver the E3.2 slice of the
[architecture](/design/fd-handoff/architecture0.gpt6a.md); the same session
then integrated the independently implemented rendezvous crates behind
facade features in `a4dfb8f2`. This checkpoint report was written by a
separate documentation session (`opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB`)
that read the commits, repaired four strict-rustdoc doc-comment defects, and
re-ran none of the implementation's broad checks. It owns documentation and
those comment repairs only. When that session reached its provider limit, the
revising Sol session (`opencode://ses_ec6b8650fffeM5meMdn7oL2CZs`) preserved
its complete draft and partial correction, strengthened the boundary test,
and committed the correction separately as `a1863146`. No optional inspection
adapter implementation is claimed by this report.

## Notify: submission, never acceptance

`systemd/notify` is runtime-free: no timer, no supervisor, no service
launch, no PID spoofing, no environment removal. A `Snapshot::capture()`
copies `NOTIFY_SOCKET` read-only; an endpoint is an absolute filesystem path
or a Linux `@abstract` address (native bytes preserved; VSOCK rejected;
empty means absent, not delivered). `send` composes one connected unbound
datagram per call — submission is **kernel enqueue only**. NotifyAccess,
unit attribution, and manager processing are separate concerns; Linux
managers identify the sender via `SO_PASSCRED` kernel credentials, and a
test verifies the manager receives `SCM_CREDENTIALS` matching the actual
sender.

- **Explicit outcomes**: `Submission::{NoManager, Submitted}` — an absent
  manager is a first-class non-error result, and `Submitted` claims nothing
  about processing.
- **Validation before send**: messages are 1–4096 bytes, NUL/CR-free,
  newline-separated `KEY=VALUE` assignments with nonempty keys; descriptor
  lists obey the rights primitive's ceiling. `BARRIER=1` must travel alone
  with exactly one descriptor. `ready`, `stopping`, `watchdog` (one
  heartbeat; cadence and eligibility are application-owned — no background
  timer), and single-line bounded `status` are conveniences over `send`.
- **No retries; errors are not proof**: calls do not retry; an error after a
  send attempt does not prove non-delivery — keep originals and verify
  custody independently before deciding recovery. Unknown assignment
  semantics stay caller-owned.
- **Timeouts**: the configured send timeout (and each barrier timeout) must
  be positive, finite, and representable; timeouts bound send/wait
  operations, not hard-real-time scheduling.

## Barrier: a bounded pipe protocol with a known-support requirement

`barrier(timeout)` creates a `CLOEXEC|NONBLOCK` pipe, sends exactly
`BARRIER=1` with the write end as its single descriptor, **drops its own
writer** (the manager's copy must be the only writer left), then polls with
a recomputed remaining-time timespec until the manager closes its copy —
`PipeClosed` on EOF. Manager-written data is `InvalidData`; expiry is
`TimedOut` with processing/acceptance explicitly unknown; `EINTR` recomputes
and continues; the deadline includes the submission itself. The fake-manager
tests pin the negative space: a manager that keeps the writer alive causes
the timeout (and the received descriptor stays valid until the manager drops
it), and unexpected pipe data is rejected.

**A barrier orders; it never accepts.** Under a manager that supports
`BARRIER` (systemd ≥ 246) it orders previously submitted notifications. It
does not prove FDSTORE acceptance or removal, and pipe closure alone cannot
detect an older manager that ignored the command and merely closed the
received descriptor — knowing the manager's capability set is a caller
requirement, which is why the fdstore crate surfaces `Capabilities` data
without negotiating anything.

## Fdstore: borrowed receipts and explicit verification

`systemd/fdstore` layers named store operations on a borrowed `Notifier`.
`FdName` follows systemd's `fdname_is_valid`: ≤ 255 printable ASCII bytes
excluding `:`, empty allowed, spaces/`=`/reserved tags allowed — names are
tags, never identities, and duplicates are valid. `Store::offer(name,
descriptors, poll)` requires at least one descriptor and sends one
`FDSTORE=1\nFDNAME=…` message (optionally with `FDPOLL=0` under
`PollPolicy::Disabled`; the default omits `FDPOLL`, leaving HUP/ERR removal
to manager policy): **one message applies one name to all its descriptors**.
`Store::remove(name)` sends `FDSTOREREMOVE=1\nFDNAME=…`, which targets *all*
manager entries carrying that tag — not one actor's offer; different names
need separate, non-atomic messages.

Receipts are **borrowed**: `StoreReceipt` keeps the original `BorrowedFd`s
(tied to the caller's lifetime) — original references remain caller-owned.
`verify_with(verifier)` is the explicit hook: `NoManager` submissions bypass
the verifier entirely; otherwise the application supplies the check and gets
back `Verification::{Unverified, Unsupported, Confirmed, Rejected}`. The
crate's own contract states the limit plainly: receipt verification is only
as strong as the supplied verifier's unit, identity, capacity and permission
checks — the built-in claim is nothing. **No verifier adapter ships in this
slice**; the tracker records one being implemented separately to complete
acceptance while the primitive stays D-Bus-free. `ManagerReport`,
`Capabilities` (`Support::{Unknown, Supported, Unsupported}` per store /
remove / poll-control / inspection / barrier), `Capacity::can_fit`
(snapshot arithmetic, not a reservation), and `Environment::maximum_hint`
(the `FDSTORE` variable, systemd ≥ 254: a startup maximum, never occupancy
or acceptance evidence; absence is unknown, not disproof) are caller-populated
data, not negotiations. Version floors — store ≥ 219, remove ≥ 236,
FDPOLL/BARRIER ≥ 246 — are documented, not probed.

No replay or generation coordination, no automatic pruning, no polling loop,
and no D-Bus/busctl dependency exist in these primitives; manager
deduplication and service-store lifetime are manager policy.

## Rendezvous integration

`a4dfb8f2` adds the two rendezvous crates (implemented and reported by
`opencode://ses_ec6d652c3ffespR5CgFSQ5uwz6`; see the
[rendezvous report](/doc/rendezvous/implementation0.glm53max.md)) to the
workspace behind `rendezvous` (the std-only contract crate) and
`rendezvous-local` (the sample explicit/env/XDG/local policy, which depends
on the contract crate). Neither feature enters `local`. The essential
exposure: resolution yields **unverified candidates** — configuration, never
reachability; no `stat`, no liveness probe, no connection, and discovery
never silently starts a server. First-yield termination prevents fallback to
a differently configured endpoint. A candidate differs from a live server
exactly there: connecting, checking liveness, and interpreting failure are
caller-owned.

## Dependency boundary

`systemd/notify` adds only the workspace rights primitive plus
rustix (`std`, `event`, `pipe`) behind `cfg(unix)`; `systemd/fdstore` depends
on notify alone (rights is dev-only). The rendezvous crates are std-only.
`Cargo.lock` grows by workspace entries only — no new registry packages —
and the facade keeps `local` at exactly `["invocation", "stdio"]`.

## Verification

Toolchain: the repo-pinned mise-selected `nightly-2026-09-30`; no stable/MSRV
checks, per the recorded policy.

Attributed to the implementing Sol session
(`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`; reported by it, not re-run
here): workspace test run of **76 passing** — 44 prior, plus 5 notify
fake-manager tests, 4 fdstore fake-manager tests, 2 fdstore unit tests, and
21 rendezvous tests (9 contract, 12 policy; validated standalone by their
implementing session per its report) — along with clippy, formatting, doc,
and feature checks on the current pinned nightly. The arithmetic is
corroborated statically here. The fake-manager tests bind real datagram
sockets (filesystem and Linux abstract) and receive through the E2.1 rights
primitive, including queue-exhaustion and credential cases; they are
**fixtures, not live systemd runs** — no real manager, unit, or NotifyAccess
policy participates.

This checkpoint's own verification: read `29271d39` and `a4dfb8f2` against
the maintained claims; repaired the four strict-rustdoc defects Sol reported
— two unresolved cross-crate intra-doc links in
[rendezvous/local](/rendezvous/local/src/lib.rs) (`Endpoint`,
`Environment` now link their defining crate explicitly), and bracketed
index text parsed as links in [invocation](/invocation/core/src/lib.rs)
(`argv[0]`) and [transport-unix](/transport/unix/src/lib.rs)
(`manifest.resources[i]`), now escaped — then confirmed
`RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --all-features --no-deps`
passes clean on the pinned nightly, with `cargo fmt --check` clean on the
three touched crates. All repairs are doc-comment-only; no code semantics
changed.

The continuation's own checks (not a new workspace-wide test claim), all on
`nightly-2026-09-30`:

- `cargo +nightly-2026-09-30 nextest run -p handle-off-systemd-notify -p handle-off-systemd-fdstore`:
  **12 passed**, including six notify integration tests, four fdstore integration
  tests, and two fdstore unit tests.
- Targeted `clippy --all-targets -- -D warnings` and `fmt --check` for notify
  and fdstore passed.
- `RUSTDOCFLAGS=-Dwarnings cargo +nightly-2026-09-30 doc --workspace --all-features --no-deps`
  passed; formatting checks for invocation, transport-unix and rendezvous-local
  passed as well.

The new boundary test accepts and receives an exact 4096-byte `STATUS=`
datagram, rejects one extra byte with `InvalidInput` and proves the configured
manager queue remains empty. No-manager validation also rejects oversized raw
and STATUS messages. A raw oversized datagram is rejected by the realistic
4096-byte fake-manager receive buffer as truncated. This fixes the review's
P2 mismatch with systemd's `NOTIFY_BUFFER_MAX=PIPE_BUF`; it does not turn a
valid submission into acceptance evidence.

The continuation committed the two index-escape comment repairs separately
in `446304f4`. The two rendezvous cross-crate link repairs were already present
inside the concurrent resolver author's larger comment revision; they were
preserved and verified but left for that author's checkpoint rather than
committing another session's intertwined work.

## Acceptance accounting: handoff-systemd-fdstore

Acceptance criteria (architecture E3.2): "Store/remove/barrier operations;
fake-manager checks; capacity and unsupported verification surfaced
explicitly. Isolate any D-Bus/busctl dependency from notification-only
builds."

- **Store/remove/barrier operations** — `Store::offer`/`remove` with
  borrowed receipts and explicit poll policy; `Notifier::barrier` with the
  bounded pipe protocol above.
- **Fake-manager checks** — ordering with a barrier acknowledgment,
  barrier-timeout and unexpected-data cases, malformed messages, queue
  exhaustion, no-manager behavior, named multi-FD offers preserving
  originals, and abstract-socket credential delivery.
- **Capacity and unsupported surfaced explicitly** — `ManagerReport`,
  `Support::Unsupported` and `Verification::Unsupported`, `Capacity` with
  overflow-checked arithmetic, and the environment maximum hint, all as
  caller-populated data.
- **D-Bus isolation** — no D-Bus/busctl dependency exists anywhere in the
  primitives; notification-only builds pull notify alone, and the planned
  inspection/verifier adapter is recorded as a separate addition.

Ticket status at documentation time: `in_progress`, with review underway and
the optional manager inspection/acceptance adapter being implemented
separately to complete acceptance. The rendezvous ticket is likewise still
open pending its own review; neither closure is claimed here.

## Later inspection checkpoint

The adapter described above as separate work landed in `420c5b76` after this
primitive checkpoint. See ⇄ [optional inspection](/doc/systemd/inspection0.gpt61s.md)
for explicit service-incarnation checks, bounded busctl queries, and offer/removal
observation. `Confirmed` there means per-name kernel-metadata membership, not
OFD identity, alias multiplicity, capacity reservation or lifecycle permission.
Notify/fdstore remain inspector-free; barriers remain ordering only. Earlier
pending-adapter and ticket-status statements above describe their checkpoint,
not a current claim that the adapter is absent. The inspection report records
the review's P2 early-JSON-entry-bound correction (`00550c3b`, confirmed by
source-only review) and further cross-step incarnation/access-flag limits.
The parent closed the E3.2 fdstore ticket after that confirmation; the earlier
in-progress status remains checkpoint history, not current ticket state.

## Links

- Direction and remaining capabilities:
  [architecture](/design/fd-handoff/architecture0.gpt6a.md)
- Coordination and toolchain policy:
  [work map](/design/fd-handoff/work.md#blocking-descriptor-transport-underway)
- Composed primitives: [rights](/doc/transport/rights0.glm53max.md),
  [state carriers](/doc/state/carriers0.glm53max.md),
  [activation](/doc/systemd/activation0.glm53max.md),
  [bundle delivery](/doc/transport/bundles0.glm53max.md),
  [rendezvous](/doc/rendezvous/implementation0.glm53max.md)
- Maintained exposure: [README — manager custody](/README.md#manager-custody)
