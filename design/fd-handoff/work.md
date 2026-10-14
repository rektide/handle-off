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
