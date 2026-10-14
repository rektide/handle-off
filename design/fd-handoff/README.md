---
type: Index
title: Handle-off design and evidence
description: Reading map for the resource-handoff toolkit proposal and recovered bus-proxy evidence.
resource: /design/fd-handoff/README.md
status: draft
generated: { by: "model:openai/gpt-6-astra", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_e9ab1e8adffeMXpHDA7AZzAeRm"
    title: Linked the remote-resource design wave covering credential URIs, SlateDB, filesystem records and proposed epics
  - resource: "opencode://ses_ec5b335ffffeIyNK21aCZL3jEw"
    title: "Added names-first systemd adoption design and mutable-state steering: application schemas, optional manifests, persistent backing, and completeness choices"
  - resource: "opencode://ses_ec616a6eaffe0I3U3HgN4tJ0Ho"
    title: "Added current naming and transcript-grounded capability opportunity assessment"
  - resource: "opencode://ses_ec7f40693ffew22J16HkhJkhFP"
    title: "Architecture direction, draft epic/ticket map, and corpus navigation"
---

# Handle-off design and evidence

## Proposed architecture

- [Systemd-native inventory and application-shaped adoption](systemd-native-adoption0.gpt6a.md):
  proposed names-first higher-level use; fixed roles and independent images need
  no wire manifest, while exact dynamic continuity retains explicit completeness
  and authority. Covers persistent immutable/mutable backing, update tradeoffs,
  exploratory SQLite/VFS questions, and coexistence with current manifest/direct
  adapters; design only, not new acceptance.
- [Cooperative lifecycle checkpoint](lifecycle0.gpt6a.md): custody versus execution
  permission, direct control channel, manager generations, and alias limitations.
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
- [Handover core report](../../doc/handover/core0.gpt61s.md): cooperative authority
  transitions, retained failures, scoped approvals, and trusted-application limits.
- [Direct handover report](../../doc/handover/direct0.gpt61s.md): separate successor
  control, same-connection process evidence, and explicit unwind/coverage limits.
- [Generation catalog report](../../doc/systemd/generations0.gpt61s.md): bounded
  manager tags, borrowed multi-generation discovery, and manifest associations.
- [Rendezvous report](../../doc/rendezvous/implementation0.glm53max.md): pure
  endpoint resolution and sample policy, with collision-free flat naming.
- [Architecture draft 0](architecture0.gpt6a.md): composable crate families,
  handle/bundle/state vocabulary, cooperative continuity, two proving
  applications, and draft epics/tickets. Current proposal for review.

## Investigations

- [Addressable resources, credentials and local realizations](../remote-resources/draft0.gpt6a.md):
  extends names-first adoption to URI-addressed resources; proposes tiny
  credential references, filesystem layouts, SlateDB/RustFS experiments and
  an epic map toward table formats and local checkouts. Design for discussion.
- [Named resources and possible futures](naming-opportunities0.gpt6a.md): current
  naming scopes; transcript-grounded opportunities for readable inventories,
  Varlink reporting, persistent datasets, restart assessment, and LUO recovery.
- [Connected-client continuity recovery](research1.glm53max.md): finds the
  implemented deployment continuation and its live evidence; corrects the
  initial investigation's checkout scope. Read first for bus-proxy precedent.
- [Initial fdstore research](research0.glm53max.md): descriptor terminology and
  manager mechanics, initially based on the listener-only public checkout.
  Read with the corrections in the recovery report.
