---
type: Index
title: Stream tools research and implementation map
description: Source-grounded storage, fixture and journal precedents for the experimental stream tools.
resource: /design/streams/README.md
status: experimental
generated: { by: "model:openai/gpt-6-astra#medium", at: "2026-10-22" }
sources:
  - resource: "opencode://ses_e9afd800bffeVHLHIvV9eCKpzQ"
    title: Commission and integrate researcher-written accounts and experimental reader tools
  - resource: /design/streams/storage0.glm53m.md
    title: Source-inspected upstream storage representations and mapping questions
  - resource: /design/streams/nodeon0.gpt6l.md
    title: Implemented native fixture tools and proposed Nodeon refinements
  - resource: /design/streams/journal-prior0.gpt6l.md
    title: Recovered prior journal research and existing reader and fixture precedents
---

# Stream tools reading map

The [experimental tools home](/stream/README.md) owns the build sequence and
domain boundaries. This map connects research and implementation evidence;
source inspection is distinct from a running fixture result.

## Storage and identity

- [Existing Durable Streams storage](storage0.glm53m.md): concrete Caddy and
  TypeScript layouts, metadata, offsets, forks and retention. Explains why a
  stream's physical backing and its public resume position must be distinguished.
- [Alternative published Rust server](crate-fixture0.gpt6a.md): verified
  `durable-streams` 0.1.5 archive, actual sealed-segment/placement manifest and
  a bounded second-fixture proposal; binary-only, not an embeddable library.
- [Addressable resources](/design/remote-resources/draft0.gpt6a.md): existing
  resource/locator/kind vocabulary and explicit resolution context; a precedent
  for stream references rather than another identity system to replace.
- [Names and retained resources](/design/fd-handoff/naming-opportunities0.gpt6a.md):
  names-first composition, immutable images and mutable backing boundaries.

## Fixture and journal precedents

- [Nodeon tools](nodeon0.gpt6l.md): implemented catalog/scenario tools,
  installed-CLI verification, retained attempts and their reuse limits. Separates
  working APIs from proposed packet/bundle/profile capabilities.
- [Nodeon review addendum](nodeon-review0.gpt6l.md): independent Luna clarification
  of definition-order execution, fresh-packet ownership and bounded report reads.
- [Recovered journal research](journal-prior0.gpt6l.md): native session references
  and existing Cyclrs, Phyllited and Umbriel work. Grounds exact-cursor checks,
  bounded replay and journal-format fixtures while preserving native ownership.

## Work anchors

| Ticket | Deliverable |
| --- | --- |
| `handoff-streams-storage` | Source-grounded storage explanation and mapping questions |
| `handoff-streams-model` | Selected stream/resource representation and resume semantics |
| `handoff-streams-workbench` | Rust server fixture and enduring HTTP CLI readers |
| `handoff-streams-journal` | Journal cursor/reference tools and reproducible fixtures |
| `handoff-streams-crate-fixture` | Alternative `durable-streams` crate assessment |
| `handoff-streams-forks` | Shared-prefix design, independently prioritized |

General grants, deadlines and lease agreements remain under
`handoff-coordination-shared-readers`; future integrations are under
`handoff-integrations`. Implementation reports will be linked here as their
verified results return.
