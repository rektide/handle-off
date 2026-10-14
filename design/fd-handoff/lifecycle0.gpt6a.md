---
type: ArchitectureAddendum
title: Cooperative handover — authority, custody, and generation boundaries
description: Implementation checkpoint decisions for the ownership coordinator and subsequent direct/systemd adapters, including actual manager verification limits.
resource: /design/fd-handoff/lifecycle0.gpt6a.md
status: draft
generated: { by: "model:openai/gpt-6-astra", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec7f40693ffew22J16HkhJkhFP"
    title: "Parent architecture decisions and core implementation authorization after custody reviews"
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: "Implementation-grounded coordinator plan and discovery of fdstore deduplication/naming constraints"
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: "Inspection review returned metadata-identity and cross-call incarnation limitations"
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: "Original capability and ticket map"
  - resource: /design/fd-handoff/research1.glm53max.md
    title: "Bus-proxy cooperative ownership precedent"
---

# Cooperative handover checkpoint

This addendum narrows implementation decisions for E4.1 and the subsequent
adapters. It does not claim those adapters are implemented. The source sessions
hold the implementation-grounded plan and review findings; the parent selected
the following coherent first slice.

## Separate custody from permission to operate

The direct transport receipt means the receiver reported acceptance of resources.
It does not mean the old owner has relinquished execution authority. An application
using handover must reconstruct the successor **dormant**, then wait for separate
release permission. Validation must avoid shared-state changes such as seeking,
changing O_NONBLOCK, shutting down a socket, or writing protocol bytes.

Core coordination is runtime/backend/codec-independent. It tracks a generation,
prepared custody, abort clearance, irreversible relinquishment, and release.
Applications supply the actual park/capture/restore behavior. Tokens can enforce
sequencing among cooperating callers, not prevent I/O via external FD aliases.

Keep original resources and the application's parked state in a long-lived
controller. Dropping an attempt does not resume, silently clean backend custody,
or restore a reusable authorization state. Ambiguous failure retains quarantine
and the evidence needed for deliberate recovery. Fallible cleanup is explicit.

Relinquishment has at least three outcomes: not relinquished, irreversibly
relinquished, and indeterminate. A plain callback error cannot authorize resume.
Once relinquishment is irreversible, a lost release message is a committed
failure, not grounds to resume the old owner.

## Direct adapter decision

Use a **separate caller-supplied control socket** initially. The reviewed bundle
protocol remains about delivery; it does not gain hidden execution semantics.
The cooperative successor acknowledges readiness while dormant, then observes
generation-correlated RELEASE or ABORT. Abort clearance covers application-made
copies as well as the received transport inventory. Lost or ambiguous retirement
acknowledgment leaves the old owner parked.

## Systemd generation decisions

- One root manifest per generation; multiple bundles and independent state images
  remain supported within it.
- Bounded backend namespace/generation tokens, rather than arbitrary application
  names, form manager tags. Resource slot suffixes bind to the manifest inventory;
  they are transient transport addresses, not logical member identities.
- Discovery returns candidates. It does not choose newest, merge ambiguous
  generations, or prune unrecognized entries.
- Publish resources and root while parked; verify the complete prepared set.
  **Publish a sealed commit marker only after irreversible relinquishment.**
  A complete root alone never authorizes successor I/O.
- Retire the selected generation's transient manager references explicitly before
  resumed I/O, with marker retirement first. Persistent listener custody stays
  **outside the transient v1 generation**.
- No magic exit status or process launcher in the core; the application selects
  replacement behavior.

### Evidence and aliases

The optional inspector's metadata match is not exact OFD identity. Its current
target pins MainPID; its snapshot brackets InvocationID only within one call.
A transaction must pin canonical unit, MainPID **and InvocationID across steps**
and retain the snapshot evidence. Do not promote identity-discarding convenience
results directly into lifecycle release authorization.

Initial manager publication assumes one cooperating publisher owns its namespace,
fresh generated names are absent at baseline, and each resource name is offered
once with one descriptor. Capacity is observed, not reserved. Resource-class and
access/status checks need explicit adapter policies; arbitrary OFD-sensitive
recovery is not implied by matching device/inode data.

The implementation plan reports upstream fdstore deduplication before new name
assignment, including resources already retained under other names. The adapter
must verify this behavior against upstream source/tests as part of its work.
Two stdio roles referring to one open resource should share a declared
`ResourceId`. **Do not infer shared OFDs from inode equality.** Undeclared aliases
that cause missing names fail preparation before commit. Strong local alias
inspection (for example F_DUPFD_QUERY where supported) is separate optional work,
not an implicit requirement of every client or an invented cross-platform fallback.

## Landing and verification

1. Core state/permission transitions, retained failures, and synthetic two-owner
   tests: abort ambiguity, dropped attempts, and post-relinquishment release errors.
2. Non-destructive systemd generation naming/catalog and manifest associations.
3. Publication/retirement with pinned evidence, explicit journal and marker.
4. Direct dormant-successor control adapter.
5. Thin CLI and bus-proxy application capture/restore integrations.

The important property is never granting both old and successor execution
permission through supported cooperative transitions. Real application quiescence,
same-resource continuity, and manager behavior need integration evidence beyond
the core state-machine tests.

Related: [architecture](architecture0.gpt6a.md), [active work map](work.md), and
[recovered bus-proxy precedent](research1.glm53max.md).
