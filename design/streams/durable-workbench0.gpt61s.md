---
type: ImplementationReport
title: Durable Streams HTTP workbench and enduring file readers
description: Research, implementation boundaries and observed fixtures for a pre-alpha HTTP workbench.
resource: /design/streams/durable-workbench0.gpt61s.md
status: draft
generated: { by: "model:openai/gpt-6-astra#medium", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a2eba1effeABgmzwRldpSIfR"
    title: Commissioned reader implementation, protocol inspection and fixture assessment
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Locator versus identity and explicit connection context
  - resource: /design/fd-handoff/naming-opportunities0.gpt6a.md
    title: Scoped names and resource continuity
  - resource: "https://github.com/thesampaton/durable-streams-rust/tree/f2db77f65f4d9d6a88b94a77b72cc9d678a2351a"
    title: Locally inspected server CLI, HTTP headers and read response implementation
---

# Durable Streams workbench

The commissioned filename is retained; the actual implementing model is Astra,
as recorded above. This report begins before implementation.

## Research and intended boundary

A locator does not establish continuity. The inspected Rust server has no
incarnation header: its GET ETag is constructed from start/end offsets (and a
closed marker). It must not be used as a generation identifier. An explicit,
operator-maintained incarnation string will bind reader state to the intended
stream generation. Delete/recreate at the same URL requires a new value.

Use direct HTTP rather than the upstream client. The server exposes PUT create,
POST append, HEAD metadata and GET with `offset`; reads supply
`Stream-Next-Offset`. Offsets remain opaque strings, including the initial `-1`
sentinel. Forks use `Stream-Forked-From` and optional `Stream-Fork-Offset`.

A single finite read buffers a complete response up to a configured byte bound.
Oversized responses fail before output and checkpoint advancement. Output is
written and flushed before a temporary state file is synced, renamed and its
directory synced. A crash in between can replay output. This is not exactly-once
delivery or proof that a downstream process durably consumed stdout.

Reader state binds schema version, normalized HTTP URL, declared incarnation,
expected media type, response byte bound and output profile. A separate advisory
lock serializes processes sharing a checkpoint. Independent state paths are
independent readers. Status and explicit reset are useful inspection/recovery
operations; no background reconciliation or cleanup is planned.

## Planned verification

Deterministic HTTP fixtures will cover resume, independent readers, binding
mismatch, response bounds and output failure. A local real-server probe will
exercise create/append/read, filesystem restart and fork retention if supported.
Observed outcomes will be appended here with exact commands and boundaries.
