---
type: Index
title: Handle-off design and evidence
description: Reading map for the resource-handoff toolkit proposal and recovered bus-proxy evidence.
resource: /design/fd-handoff/README.md
status: draft
generated: { by: "model:openai/gpt-6-astra", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec7f40693ffew22J16HkhJkhFP"
    title: "Architecture direction, draft epic/ticket map, and corpus navigation"
---

# Handle-off design and evidence

## Proposed architecture

- [Active work map](work.md): native epics, retained legacy tickets, and the
  authorized implementation starting point.
- [Foundation implementation report](../../doc/foundation/implementation0.gpt61s.md):
  delivered APIs, extraction owners, feature graph, and measured local baseline.
- [Architecture draft 0](architecture0.gpt6a.md): composable crate families,
  handle/bundle/state vocabulary, cooperative continuity, two proving
  applications, and draft epics/tickets. Current proposal for review.

## Investigations

- [Connected-client continuity recovery](research1.glm53max.md): finds the
  implemented deployment continuation and its live evidence; corrects the
  initial investigation's checkout scope. Read first for bus-proxy precedent.
- [Initial fdstore research](research0.glm53max.md): descriptor terminology and
  manager mechanics, initially based on the listener-only public checkout.
  Read with the corrections in the recovery report.
