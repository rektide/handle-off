---
type: ImplementationReport
title: Bounded systemd generation addresses and borrowed catalogs
description: Partial E3.3 naming/discovery checkpoint, manifest-slot associations, preserved caller inventory, scan bounds and upstream deduplication evidence.
resource: /doc/systemd/generations0.gpt61s.md
status: implemented
generated: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6b8650fffeM5meMdn7oL2CZs"
    title: Read naming/catalog source and tests plus upstream deduplication paths; documented partial scope without broad reruns
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented 36321247 and reported nine new tests, 105 workspace tests and current-nightly checks via parent dispatch
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: Source-only catalog review confirmed grammar, bounds, borrowed mapping and dedup claims without substantive findings, via parent dispatch
  - resource: /systemd/generation/src/lib.rs
    title: Addressing/discovery-only contract and deferred execution checks
  - resource: /systemd/generation/src/name.rs
    title: Typed tokens, fixed tag grammar and bounded parsing
  - resource: /systemd/generation/src/catalog.rs
    title: Borrowed inventory, candidates, scan limits and manifest mapping
  - resource: /systemd/generation/tests/names.rs
    title: Round-trip and malformed-name boundary tests
  - resource: /systemd/generation/tests/catalog.rs
    title: Multiple candidates, preservation, aliases, missing/extra slots and metadata/scan limits
  - resource: /systemd/generation/Cargo.toml
    title: Foundation-only dependencies with no backend/codec/RNG selection
  - resource: /handle-off/Cargo.toml
    title: Optional systemd-generation feature independent of local
  - resource: file:///home/rektide/archive/systemd/systemd/src/core/service.c
    title: Independently read service_add_fd_store capacity/deduplication/name order (lines 662–701)
  - resource: file:///home/rektide/archive/systemd/systemd/src/basic/fd-util.c
    title: Independently read same_fd F_DUPFD_QUERY/kcmp and inode/open-flag fallback (lines 466–551)
  - resource: /design/fd-handoff/lifecycle0.gpt6a.md
    title: Backend token, generation mapping, deferred publication and alias decisions
  - resource: /doc/handover/core0.gpt61s.md
    title: Separate cooperative permission core, never implied by catalog presence
  - resource: /doc/systemd/inspection0.gpt61s.md
    title: Metadata and cross-call incarnation limitations for future verification adapters
  - resource: /rust-toolchain.toml
    title: Current pinned nightly-2026-09-30
---

# Bounded generation addresses and borrowed catalogs

Commit `36321247`, by `opencode://ses_ec717edaeffeP56i2EfWqe8r3v`, adds
`handle-off-systemd-generation` under `systemd/generation`, facade feature
`systemd-generation` and reexport `systemd_generation`. It implements **only
the naming/catalog subset of E3.3**, not publication, retirement, manager
queries, adoption or end-to-end handover. Independent source-only review reported
no substantive findings; E3.3 completion is not claimed.

## Fixed backend addressing

`NamespaceToken` and `GenerationToken` are caller-provided 128-bit values,
constructed from bytes/u128 or parsed from exactly 32 lowercase hex digits.
They are backend coordinates, **not arbitrary application logical IDs**.
The crate neither generates randomness nor hashes, truncates or normalizes
names. Namespace exclusivity and generation uniqueness are caller policy;
numeric token order expresses no freshness or authority.

`GenerationKey` pairs the tokens; `Address` pairs the key with one `Member`:

| Member | Canonical tag | Length |
| --- | --- | --- |
| `Root` | `ho1.<namespace32hex>.<generation32hex>.m` | 71 ASCII bytes |
| `CommitMarker` | `ho1.<namespace32hex>.<generation32hex>.c` | 71 ASCII bytes |
| `Resource(ResourceSlot)` | `ho1.<namespace32hex>.<generation32hex>.rXXXXXXXX` | 79 ASCII bytes |

The resource suffix is exactly eight lowercase hex digits encoding a u32
slot. All generated tags fit systemd's 255-byte FDNAME bound, excluding `:`
and control bytes. `Address::parse` checks fixed widths/delimiters without
copying strings or allocating by slot number. Uppercase, oversized tokens,
bad delimiters and malformed roles are errors, not silently normalized.

Names outside `ho1.` are unrelated and preserved. Bare `ho1` and malformed
names inside the reserved family fail discovery even if their namespace is
not the caller's selected namespace. Unknown future families such as `ho2.`
remain unrelated; the crate never invents a compatibility parser for them.

## Borrowed, non-destructive discovery

`Catalog::discover` takes `NamedHandle { name: Option<&str>, handle: &H }`
inputs. `H` is opaque caller-owned data: it may wrap an activation owner or
another already-valid handle, but the catalog never claims, closes, duplicates
or probes it. The caller keeps names and handles alive for the borrow.

The catalog exposes all keyed `Candidate`s, exact-key lookup, a namespace
filter and `unrelated()` entries. References preserve original input indices;
unrelated names (including unnamed entries and duplicate ordinary tags) remain
borrowed without copies. Persistent listeners stay outside this transient
generation family. Multiple namespaces and generations coexist; deterministic
BTree ordering is presentation, **not newest-generation selection**.

A repeated recognized root, marker or resource address fails with `Duplicate`,
even when the supplied handles are equal. Malformed names and limit errors
drop only borrowed catalog state, leaving caller inventory intact. Successful
catalog Drop is likewise non-destructive. There is no automatic prune,
reconciliation, candidate merge or fallback adoption.

## Mapping manifests and declared aliases

`Candidate::map_manifest(&Manifest)` requires a root, bounded metadata, valid
manifest semantics and **exact contiguous resource slots** from zero through
`manifest.resources.len() - 1`. Slot `i` binds to `manifest.resources[i]`,
independent of input FD order/numbers. Missing/extra inventory and holes are
errors. `ManifestMapping` borrows the root, optional marker, logical generation
and each ResourceId/handle association; it owns no original resources.

The caller must establish that the supplied decoded manifest actually belongs
to that root. This crate reads no root bytes and does not encode a logical
`GenerationId` into a backend token or prove their relationship. Manifest
validation checks declared membership, not resource kind, application extension
compatibility or actual custody.

Alias roles share references to one declared `ResourceId`, binding that resource
once. Duplicate ResourceIds in the inventory fail semantic validation; equal
inodes or FDNAMEs never infer aliases or shared OFDs. Multiple bundles and
independent state-image resources remain expressible through foundation types.

A prepared mapping may lack a commit marker. `require_marker()` establishes
only physical marker presence; **neither root presence, marker presence nor
successful mapping grants execution permission**. Later adapters must validate
sealed root/marker contents and linkage, complete custody, canonical service
incarnation across steps, compatibility and transient retirement before resumed
I/O. These checks are not implemented by discovery. The separate
[handover core](/doc/handover/core0.gpt61s.md) owns cooperative sequencing, not
implicit authority from catalog names.

## Limits and sparse indexing

Default scan limits are **4096 total input entries** and **256 generations**;
both configurable ceilings are 65,536. Entries include unrelated inputs.
Limits are checked during iteration before storing another entry/candidate;
the caller still controls its iterator and preexisting input allocations.
No elapsed-time or arbitrary iterator-work guarantee is implied.

Resources use a sparse BTreeMap keyed by `ResourceSlot`: a lone `u32::MAX` slot
allocates one entry, not an array indexed up to that number. Mapping later
rejects it when slot zero is required. Manifest mapping caps resources and
counted metadata nodes at 65,536 before semantic validation/binding allocation.
Nodes count resources, bundles, members and extension entries; these are not
exact heap/string-byte limits. Borrowed unrelated names do not expand into
copied strings merely because they are long.

## Upstream deduplication before naming

This documentation session independently read the archived upstream source:

- [`service_add_fd_store`](file:///home/rektide/archive/systemd/systemd/src/core/service.c)
  checks capacity first (678–681), then compares existing entries using `same_fd`
  (683–690), and only afterwards allocates an entry and assigns the supplied
  name (693–701). A suppressed duplicate returns without acquiring the new name.
  Even a duplicate can first encounter the full-store capacity rejection.
- [`same_fd`](file:///home/rektide/archive/systemd/systemd/src/basic/fd-util.c)
  prefers `F_DUPFD_QUERY`, then same-process `kcmp(..., KCMP_FILE, ...)`. When
  unavailable or denied it falls back to `fstat` inode identity plus equal
  `F_GETFL` open-status flags, excluding character/block devices. The fallback
  can equate separate opens of one file; it is not exact OFD proof. These
  source paths propagate other errors rather than always guessing equality.

Consequently a duplicate already stored under a different tag may not receive
a generation resource-slot tag. Later publication verification must detect
missing names **before commit**, not infer successful naming from submission or
silently repair the catalog. One logical alias resource should use one declared
ResourceId; this crate performs no local OFD-alias inspection. The observation
is grounded in the archived implementation, not a live manager mutation or a
guarantee that every deployed systemd/kernel uses the strongest comparison path.

## Dependencies and verification

Normal dependencies are the workspace foundation bundle/handles crates, with
their existing state foundation dependency. The crate selects no codec, carrier,
inspector, fdstore/notify backend, RNG, external package or async runtime. The
facade feature is optional and excluded from `local` and empty defaults.

Via parent dispatch, implementer `opencode://ses_ec717edaeffeP56i2EfWqe8r3v`
reports **105 passing workspace tests**, including **nine new generation tests**,
and checks on pinned `nightly-2026-09-30`. These are attributed implementation
results, not broad reruns by this documentation session. Two naming tests and
seven catalog tests cover round trips, malformed grammar, shuffled multi-key
inventory, duplicate addresses, absent roots/markers, exact/sparse slots,
declared alias roles, non-destructive Drop/errors and scan/metadata bounds.
Synthetic handles and drop counters are not live systemd publication or restart
evidence.

This session checked source/test API claims, read the cited upstream paths and
verified document links/provenance. Reviewer
`opencode://ses_ec6f1618effeapZzCE6xoMvdTD`, via parent dispatch, confirmed the
token grammar/bounds, borrowed preservation, sparse maps/contiguous mapping,
shared ResourceIds and upstream deduplication claims, with no substantive
findings. That review was source-only, with no test rerun. Token uniqueness,
root/manifest linkage and marker/custody contents remain caller responsibilities.
E3.3 still requires the separately planned publication/retirement and
execution-permission adapter work. No end-to-end handover or manager acceptance
result is claimed here.

## Cross-references

- [README — generation catalogs](/README.md#generation-catalogs): maintained
  exposure of the subset and its deferred capabilities.
- [Lifecycle direction](/design/fd-handoff/lifecycle0.gpt6a.md#systemd-generation-decisions):
  root/marker ordering, namespace policy and later publication/retirement work.
- [Inspection checkpoint](/doc/systemd/inspection0.gpt61s.md): metadata and
  cross-call service-incarnation boundaries for future verification.
- [Custody primitives](/doc/systemd/custody0.glm53max.md): borrowed offers and
  explicit removal/barrier operations, not a generation transaction.
