---
type: ImplementationReport
title: Cooperative handover permission core
description: Backend-free generation bookkeeping, application-owned originals, retained evidence, fail-closed transitions and attributed synthetic verification.
resource: /doc/handover/core0.gpt61s.md
status: implemented
generated: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6b8650fffeM5meMdn7oL2CZs"
    title: Read core source/tests and lifecycle direction; documented actual sequencing and retained-failure limits without broad reruns
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented 11b78c3b and reported seven new synthetic tests, 96 workspace tests, token doctests and pinned-nightly checks via parent dispatch
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: Source-only review reported no substantive safety blockers in adversarial sequencing/drop/recovery, with trusted-application limits, via parent dispatch
  - resource: /handover/core/src/lib.rs
    title: ParkingController, Attempt, evidence records, authority/custody transitions and non-Clone approvals
  - resource: /handover/core/tests/lifecycle.rs
    title: Synthetic two-owner gates, abandonment, ambiguity, panics, wrong generations and one-shot approvals
  - resource: /handover/core/Cargo.toml
    title: Foundation-only dependency through handle-off-bundle
  - resource: /handle-off/Cargo.toml
    title: Opt-in handover facade feature independent of local
  - resource: /design/fd-handoff/lifecycle0.gpt6a.md
    title: Parent-selected custody/authority separation and deferred direct/systemd adapter responsibilities
  - resource: /doc/systemd/inspection0.gpt61s.md
    title: Metadata matching and per-call incarnation limits that must not become lifecycle release proof
  - resource: /doc/transport/bundles0.glm53max.md
    title: Delivery acceptance is custody, not successor execution permission
  - resource: /rust-toolchain.toml
    title: Current pinned nightly-2026-09-30
---

# Cooperative handover permission core

Commit `11b78c3b`, implemented by
`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`, adds `handle-off-handover` under
`handover/core`, with facade feature/reexport `handover`. It implements the
first core slice of the [lifecycle direction](/design/fd-handoff/lifecycle0.gpt6a.md):
generation-scoped custody and authority bookkeeping, explicit application/backend
evidence, retained failure state and one-shot cooperative approvals.

**This is not an I/O owner, backend, supervisor or restart implementation.**
Independent source review reported no substantive safety blockers under the
cooperative contract; no ticket acceptance is claimed here. Concrete
direct-control and systemd-generation adapters, and
real application park/capture/restore integrations, remain later work.

## Ownership and the long-lived controller

The application retains its original resources and parked state in its own
long-lived parking controller, with `ParkingController<E>` alongside them.
`new_parked(generation)` attests that conflicting old I/O/admission is already
parked; it neither stops threads nor claims resources. A generation must be
unique in its ownership domain. Recreating bookkeeping to bypass quarantine
would violate the cooperative contract.

An `Attempt` holds an exclusive mutable borrow of the bookkeeping, not ownership
of the originals. `Status` keeps **Authority** separate from **Custody** and
quarantine. `records()` retains attempt IDs, transition kinds and
`Evidence<E>`, including failed or wrong-generation outcomes; transition errors
point to record indices. `abandoned_attempts()` and `last_abandonment()` retain
abandonment count and the latest authority/custody snapshot.

Dropping an unfinished attempt sets quarantine and records abandonment. It
does **not** resume old I/O, close original resources, clean backend copies,
invoke callbacks or reset permissions. Starting another attempt preserves
quarantine/history and permits only explicit state-valid recovery. Dropping
the controller itself is application disposal, not rollback or resume approval.
Original-resource retention depends on the application retaining its actual
owners; this core cannot prevent their independent destruction.

Evidence should be passive metadata, not a lease whose Drop automatically
resumes an application. Such leases must be disarmed **before publication**,
not merely before a transport acknowledgment. Retain the complete possible
publication plan before callbacks can create side effects.

## Trusted assertions, not OS enforcement

`Evidence::new(generation, value)` is public. The core checks its generation
label and sequencing, not the truth or freshness of its contents. Callback success attests
application/backend facts: complete custody, dormancy, retirement or permanent
relinquishment. It does not inspect resources, authenticate evidence or prove
quiescence. External aliases, independent actors and raw FD I/O remain outside
token enforcement.

Both old and successor must honor application gates. A successor remains
dormant after delivery acceptance; dormancy forbids conflicting shared-state
changes such as seeking, changing O_NONBLOCK, shutting down a socket or writing
protocol bytes unless rollback explicitly accounts for them. Capture sufficiency,
extension compatibility and actual backend validation remain application work.

A [bundle-delivery ACK](/doc/transport/bundles0.glm53max.md) is not release
permission. An [inspector metadata match](/doc/systemd/inspection0.gpt61s.md)
does not establish OFD identity, alias multiplicity/access flags or a pinned
incarnation across lifecycle steps. Neither may be promoted directly into a
generic "safe to close all originals" or "successor may run" claim.

## Actual transition boundary

| API | Required facts and conservative outcome |
| --- | --- |
| `publish(start, operation)` | Requires parked-old authority and local-only custody. Records start evidence and marks custody exposed **before** invoking the callback; error/panic cannot prove non-delivery. Success remains exposed, not verified. |
| `verify_custody(start, verify)` | Requires non-local custody and parked/relinquished/indeterminate authority. Reverts custody to exposed before checking; trusted success records complete generation-matched dormant custody as verified. It cannot resolve indeterminate authority. |
| `abort(start, retire)` | Allowed only while old authority is parked. Trusted success must retire **all** possible copies and restoration opportunities, with no successor permitted to execute. Issues one `ResumePermit`; errors/panics grant none. Even local-only abort uses an explicit callback. |
| `relinquish(start, operation)` | Requires parked-old authority and verified custody. Sets authority indeterminate before callback, then accepts one of three explicit outcomes below. |
| `confirm_relinquished(evidence)` | Resolves indeterminate authority only toward permanent relinquishment, never toward old resume. Generation-matched application evidence is required. |
| `release(start, operation)` | Requires relinquished authority and verified custody. Sets authority `NewReleaseIssued` **before** passing the grant to the callback; issuance is irreversible even on error/panic. |

`RelinquishOutcome` distinguishes:

- **NotRelinquished:** an affirmative no-revocation assertion restores
  `ParkedOld`, not permission to run. Verified abort is still needed for resume.
- **Relinquished:** the application has permanently disabled old execution,
  including lease Drop/shutdown paths. Originals may remain retained for recovery.
- **Indeterminate:** neither old resume nor new release is approved. A generic
  callback error must not be treated as NotRelinquished. Fresh custody evidence
  alone cannot repair authority uncertainty.

`ResumePermit` and `ReleaseGrant` have no public constructors, do not implement
Clone, and expose their generation for application checks. Dropping either has
no action. Terminal old-resume/new-release issuance prevents another attempt
from issuing another approval on that controller. Consuming a permit/grant is
an application/backend operation, not an OS capability enforced on descriptors.
Single issuance is **per controller**: the core does not enforce controller/domain
redemption, prove that an application consumed an approval correctly, or prevent
another controller from being constructed. The application must check matching
generation **and ownership domain**, consume approvals correctly, prevent resets,
police external aliases and disarm auto-resuming leases. Non-Clone tokens do not
replace those obligations.

Release failure cannot restore old authority: the grant may already have escaped
and caused successor I/O. There is no automatic retransmission or reissued grant;
any explicit idempotent retry relies on backend-retained original release data.

## Sequencing examples from the tests

These are **prose API traces**, not runnable backend examples. The actual
[tests](/handover/core/tests/lifecycle.rs) use synthetic `Gates` booleans and
trusted string evidence; they do not transfer descriptors or restart a service.

**Successful release path:**

1. Application parks old I/O, retains originals and creates
   `ParkingController::new_parked` for its generation.
2. `attempt()` borrows bookkeeping. `publish` records the complete plan and
   delivers copies to a dormant successor; an ACK leaves `Custody::Exposed`.
3. `verify_custody` attests complete generation-scoped dormant custody.
4. `relinquish` permanently disables old execution and explicitly returns
   `RelinquishOutcome::Relinquished`.
5. `release` passes its one-shot `ReleaseGrant` to the backend/application gate,
   which checks the generation and permits successor execution. A lost response
   leaves old authority surrendered even if delivery cannot be determined.

**Ambiguous publication followed by deliberate abort:**

1. `publish` may deliver copies and then return failure. Keep originals, the
   possible publication plan and failed evidence; old I/O stays parked.
2. An uncertain `abort` callback returns failure and produces no resume permit.
   Dropping the attempt quarantines the retained controller, not the originals.
3. A later explicit `attempt().abort(...)` verifies retirement of every possible
   successor copy/restoration opportunity. Only then does the application consume
   the returned `ResumePermit` to resume the matching parked generation.
4. If relinquishment was already indeterminate or irreversible, this old-resume
   path is forbidden; do not replace the controller to evade that fact.

## Dependency and verification boundaries

The crate depends only on the foundation `handle-off-bundle` for `GenerationId`
(and its existing workspace foundation dependencies). No external crate, async
runtime, serialization codec, systemd, busctl or direct-transport dependency is
introduced. The optional facade feature does not enter `local` or empty defaults.

Via parent dispatch, implementer `opencode://ses_ec717edaeffeP56i2EfWqe8r3v`
reports **seven new synthetic tests**, **96 passing workspace tests**, token
compile-fail doctests and current pinned `nightly-2026-09-30` checks. These are
attributed implementation results, **not broad reruns by this documentation
session**. The compile-fail source demonstrates that `ResumePermit` cannot be
cloned; both approval types' non-Clone shape is also visible in source.

The seven tests cover ambiguous publication/retirement, abandoned attempts with
retained originals/evidence, irreversible release failure, the three
relinquishment outcomes, panic-conservative facts, wrong-generation evidence,
and one-shot approvals with inert Drop. They establish cooperative state-machine
behavior under synthetic gates, not real application quiescence, backend
retirement or same-resource continuity across a restart.

This session read source/tests, manifests and the lifecycle direction, checked
the documented API sequencing and links/provenance, and performed no broad test
reruns. Reviewer `opencode://ses_ec6f1618effeapZzCE6xoMvdTD`, via parent dispatch,
reported no substantive safety blockers after source-only adversarial review of
callback sequencing, Drop and recovery. No tests were rerun for that review.
This conclusion applies to the cooperative transition contract, not real OS
enforcement, evidence freshness or cross-controller/domain redemption.

## Cross-references

- [README — cooperative handover](/README.md#cooperative-handover): maintained
  exposure of the implemented core and deferred backend work.
- [Lifecycle direction](/design/fd-handoff/lifecycle0.gpt6a.md): application
  responsibilities and the proposed separate direct/systemd adapter landings.
- [Foundation report](/doc/foundation/implementation0.gpt61s.md): resource
  identity and ownership vocabulary reused without choosing a backend.
