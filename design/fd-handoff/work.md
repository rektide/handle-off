---
type: WorkMap
title: Handle-off implementation work map
description: Native epic mapping, preserved legacy work, and initial implementation dispatch.
resource: /design/fd-handoff/work.md
status: active
generated: { by: "model:openai/gpt-6-astra", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec7f40693ffew22J16HkhJkhFP"
    title: "Implementation authorization, native ticket creation, FX preparation and Sol medium build coordination"
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: "Architecture and acceptance criteria materialized in beads"
---

# Active work map

The operator authorized building handle-off, with FX preparing the fresh
workspace and Sol medium implementing coherent slices. The
[architecture](architecture0.gpt6a.md) remains the design reference; native beads
items own implementation status. The existing `pfd` tracker prefix is retained.

| Draft epic | Native epic |
| --- | --- |
| E1 Foundation | `pfd-handle-off-foundation` |
| E2 Direct delivery | `pfd-handle-off-direct` |
| E3 Activation/systemd | `pfd-handle-off-systemd` |
| E4 Cooperative handover | `pfd-handle-off-handover` |
| E5 Thin CLI | `pfd-handle-off-cli` |
| E6 Bus-proxy/generality | `pfd-handle-off-bus-proxy` |
| E7 Varlink | `pfd-handle-off-varlink` |
| E8 Completion/introspection | `pfd-handle-off-completion` |
| E9 One-off startup | `pfd-handle-off-one-off` |
| E10 Application coordination | `pfd-handle-off-coordination` |
| E11 Inspection | `pfd-handle-off-inspection` |

All 35 draft child rows have native tickets, human-readable IDs, acceptance
criteria, architecture pointers, and explicit parent-child edges. Initial
prerequisites follow the draft; future capabilities remain lightly planned.
Dependency-cycle verification passed when the map was created.

## Existing work carried forward

Four open legacy undertakings retain their histories through native renames:

- `pfd-return-values` → `pfd-handle-off-cli-result` (E5.2).
- `pfd-daemon-async-send-fd` → `pfd-handle-off-coordination-async` (E10.3,
  optional integration rather than mandatory runtime).
- `pfd-daemon-extra-pipes` → `pfd-handle-off-direct-extra-handles`
  (additional named resource examples).
- `pfd-daemon-sd-notify` → `pfd-handle-off-systemd-notify`
  (service notification lifecycle alongside custody-specific work).

Closed historical work remains historical; it is not claimed as completion of
new handle-off acceptance criteria. There are 11 new epics and 37 associated
children, including the two additional legacy undertakings.

## First implementation checkpoint

FX prepares a clean, compiling domain-grouped workspace, retiring obsolete clid
code while retaining research and evidence. Sol medium then implements the
small-build foundation and independently useful contracts, with real local
stdio/invocation use and meaningful ownership/association checks. Later direct
transfer and manager integration should build on that reviewed foundation.

The parent owns architectural integration and tracker updates. External
bus-proxy deployment changes are a later consumer task, not part of workspace
preparation. Native ticket notes carry implementation reports and session links.

### Workspace preparation landed

FX committed the dependency-free facade scaffold (`75671d87`) and retired the
old clid packages/examples and stale prefork placeholders (`831e2e0f`). Cargo
check, clippy, and formatting passed. Sources referenced by the research remain
recoverable from pre-retirement tree `09437c06`; source paths inside those
historical reports should be interpreted at that revision.

Sol medium is implementing separate handle, bundle/state, stdio, and invocation
capabilities plus a local example and feature/dependency baseline. A single
combined contracts crate was considered during cleanup but not selected: the
operator's meaningful heavy-decomposition preference remains controlling.

### Foundation implementation landed

Sol medium committed five capability crates and a local example (`9c82eef1`),
then the maintained README and [implementation report](../../doc/foundation/implementation0.gpt61s.md)
(`86ca200b`). Handles, bundles, opaque state extensions, native invocation data,
and stdio roles are independent capabilities. The facade's defaults are empty;
its local feature depends only on invocation and stdio. There are no external
dependencies in this slice.

Reported checks passed: nextest (11 tests), clippy with warnings denied,
formatting, docs, and individual feature builds. The stripped dynamically linked
local example measured 355,640 bytes; the report records the reproducible method
and limits of that number. A focused ownership/contract review follows.

The remote binary/dependency comparison was separated from E1.3 into
`pfd-handle-off-direct-build-baseline`, dependent on the real CLI backend/result
work. It cannot be measured against a nonexistent transport and should not block
local-only capabilities. This adds one child to the original materialized map.

### Foundation review complete

Followup review (`opencode://ses_ec70f2b7fffePk1evnZ4cGPhJb`) found no substantive
ownership, membership, state-range, native-byte or feature-isolation issues.
It additionally verified a workspace/all-features check on stable Rust 1.93.1.
The subsequent operator direction is to use current user-system Rust via mise,
not older stable/MSRV checks. The repository now pins the mise-selected
`nightly-2026-09-30` (rustc 1.101.0-nightly, `5c543b0b8`, 2026-09-29).
The earlier stable probe is historical evidence only, not ongoing test policy.

E1 and E5.1 are closed with implementation and review evidence. Future adoption
diagnostics and transport-facing API conveniences were noted on their relevant
tickets; they do not block the foundation. Descriptor delivery, manager custody,
and restart continuity remain subsequent work, not claims of this local slice.

### Blocking descriptor transport underway

Sol medium continues with `pfd-handle-off-direct-rights`: an independently usable
blocking SCM_RIGHTS crate, real multiple-descriptor transfer, explicit borrowed
send/owned receive and truncation/CLOEXEC behavior. Runtime, codec, and bundle
contracts stay outside the primitive. Checks use the pinned mise-selected Rust.

Transport code landed as `20c4edab`; the
[transport report](../../doc/transport/rights0.glm53max.md) and README exposure
landed as `70a72bee`. The primitive uses rustix and explicitly supports Unix
datagrams, with borrowed send, owned receive, CLOEXEC policy and truncation
cleanup. Sol reported 19 passing tests plus clippy/format/feature checks. Local
consumers retain their runtime/codec/transport-free dependency graph. Focused
review completed with no substantive issues, including source-level confirmation
of rustix cleanup, ancillary-padding limit enforcement, and CLOEXEC behavior.
`pfd-handle-off-direct-rights` is closed with that evidence.

E2.2 code landed as `0c1c9a90`: separate optional rkyv representation/validation
and Linux sealed-memfd carrier implementations, with core contracts independent.
Sol reports 27 passing tests including multiple image/manifest transfer; focused
state review and documentation are underway. Independently usable multi-FD
activation adoption (E3.1) is the next implementation slice in progress.
