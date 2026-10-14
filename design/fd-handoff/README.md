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
- [Descriptor transport report](../../doc/transport/rights0.glm53max.md):
  blocking datagram rights API, ownership/truncation behavior, and dependency cost.
- [Bundle delivery report](../../doc/transport/bundles0.glm53max.md): sample
  transaction protocol, application acceptance, and multi-image process example.
- [State carrier report](../../doc/state/carriers0.glm53max.md): independent
  rkyv wire encoding and sealed memfd storage, limits, and composition evidence.
- [Socket activation report](../../doc/systemd/activation0.glm53max.md): classic
  named-descriptor adoption, explicit ownership boundary, and reviewed parser fix.
- [Systemd custody report](../../doc/systemd/custody0.glm53max.md): notification,
  borrowed storage offers, removal, barrier ordering, and acceptance boundaries.
- [Manager inspection report](../../doc/systemd/inspection0.gpt61s.md): optional
  bounded queries, named metadata verification, and incarnation/identity limits.
- [Rendezvous report](../../doc/rendezvous/implementation0.glm53max.md): pure
  endpoint resolution and sample policy, with collision-free flat naming.
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
