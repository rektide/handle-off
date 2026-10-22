---
type: Readme
title: Experimental stream readers and references
description: Pre-alpha CLI workbenches for durable stream and journal sources, file-backed reading state, and inspectable source references.
resource: /stream/README.md
status: experimental
generated: { by: "model:openai/gpt-6-astra#medium", at: "2026-10-22" }
sources:
  - resource: "opencode://ses_e9afd800bffeVHLHIvV9eCKpzQ"
    title: Commission stream and journal workbenches, reader state, source routing and model-specific research and implementation
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Existing distinction between resource identity, locator, kind, representation and resolution context
  - resource: /README.md
    title: Existing named resource, bundle, state, transport and custody boundaries
---

# Experimental stream readers and references

This is a **pre-alpha experimental subtopic** of handle-off: CLI tools and libraries
for reading event sources, retaining simple file-based progress, and describing
the source and position well enough for another reader to continue. Individual
crate READMEs own runnable commands and verified capabilities as they land.

The [research reading map](/design/streams/README.md) connects storage findings,
Nodeon tool precedents and recovered journal work.

## Organization

| Home | Responsibility |
| --- | --- |
| `stream/durable-streams/` | HTTP Durable Streams reader and external server workbench |
| `stream/systemd/` | Journal event-source reader, cursor and participant identity evidence |
| `design/streams/` | Research, fixture precedents, implementation evidence and integration decisions |

These are source adapters. Existing [handle-off primitives](/README.md) continue
to own resource IDs, bundles, state metadata, descriptor delivery and custody.
The first tools need not transfer a live server or adopt its storage engine.

## Working boundaries

- **Source reference:** describes the intended event source and explicit selection
  context. A locator is not proof of an immutable source incarnation.
- **Checkpoint:** binds a source selection to its opaque resume position and
  relevant observations. A journal cursor and an HTTP stream offset have different
  semantics; preserve each source's native token.
- **Reader identity:** distinguishes independent consumers and their checkpoint
  files. Two readers of the same source need not share progress.
- **Participant evidence:** observed machine, boot, unit, invocation or process
  identities can explain a source transition. A unit name alone does not identify
  a particular service run.
- **Delivery boundary:** output and checkpoint replacement are separate effects
  unless an implementation explicitly joins them. Report the replay/loss window;
  an atomic checkpoint file alone does not establish exactly-once delivery.

The existing [resource-record proposal](/design/remote-resources/draft0.gpt6a.md)
separates identity, locator, kind and representation. Use that distinction when
developing source references. Private URI conventions must be identified as such.

## Build and assessment sequence

1. Preserve source-grounded storage research and relevant Nodeon/journal precedents.
2. Build a Durable Streams HTTP workbench and a journal reader independently,
   with deterministic fixtures and explicit source-bound checkpoint state.
3. Exercise a real Rust Durable Streams server and a read-only local journal
   where available; distinguish these observations from fixture-only evidence.
4. Assess the two implementations together. Introduce shared reference/routing
   code only where their actual interfaces establish a useful common contract.
5. Refine journal handoff and successor detection with a separate integration pass.

General reader grants, deadlines and lease agreements are tracked independently
under `handoff-coordination-shared-readers`. Stream models and forks live under
`handoff-streams`; forks are not a prerequisite for these reader tools.

Current implementation anchors: `handoff-streams-workbench` and
`handoff-streams-journal`. `handoff-streams-crate-fixture` tracks assessment of the
separately named `durable-streams` crate as an alternative fixture. Future
StreamDB, durable-state, PicoMQ and Rivet investigations live under
`handoff-integrations`.
