---
type: ImplementationReport
title: Checked state codec and sealed Linux carrier
description: Checkpoint record of the Sol-implemented E2.2 slice covering the pinned-rkyv wire codec with bounded checked encode/decode, the mandatory-seal positional-I/O memfd carrier, their independence, dependency cost, and pfd-handle-off-direct-state-carriers acceptance accounting.
resource: /doc/state/carriers0.glm53max.md
status: implemented
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB"
    title: Documentation checkpoint verifying the state-carrier slice against its code and extending the README exposure
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented the checked state codec and sealed memfd carrier in 0c1c9a90 and reported the workspace checks recorded below
  - resource: /state/rkyv/src/lib.rs
    title: Codec envelope, budgets, bounded writer and decode/semantic validation
  - resource: /state/rkyv/src/wire.rs
    title: Private wire schema kept out of the core types
  - resource: /state/rkyv/Cargo.toml
    title: Exact-pinned rkyv 0.8.18 with fixed format features
  - resource: /state/memfd/src/lib.rs
    title: Sealed image creation, adoption validation and positional reads
  - resource: /state/memfd/tests/carrier.rs
    title: Seal/limit/offset/CLOEXEC carrier coverage
  - resource: /state/memfd/tests/composition.rs
    title: Codec, carrier and transport composition without new coupling
  - resource: /state/memfd/Cargo.toml
    title: Linux-gated rustix normal dependencies and dev-only composition deps
  - resource: /Cargo.toml
    title: Workspace membership for both state crates
  - resource: /handle-off/Cargo.toml
    title: Separate facade features isolated from local
  - resource: /handle-off/src/lib.rs
    title: Facade reexports and remaining facade-doc gap
  - resource: /Cargo.lock
    title: Registry closure after the rkyv dependency
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E2.2 acceptance criteria
  - resource: /design/fd-handoff/work.md
    title: Checkpoint coordination and toolchain policy
  - resource: /doc/foundation/implementation0.gpt61s.md
    title: Foundation contracts the codec validates against
  - resource: /doc/transport/rights0.glm53max.md
    title: Transport primitive the composition test reuses
  - resource: /rust-toolchain.toml
    title: Repo-pinned mise-selected nightly-2026-09-30
  - resource: /README.md
    title: Maintained exposure extended by this checkpoint
---

# Checked state codec and sealed Linux carrier

`handle-off-state-rkyv` and `handle-off-state-memfd`, committed in `0c1c9a90`
by the Sol medium session (`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`),
deliver the E2.2 slice of the
[direct-delivery sequence](/design/fd-handoff/architecture0.gpt6a.md): a
checked, bounded wire codec for manifests and opaque state metadata, and an
independent sealed Linux memfd byte carrier. This checkpoint report was
written by a separate documentation session
(`opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB`) that read the commit and
verified the maintained claims against the code; it re-ran none of the
implementation checks and owns no code, tickets, or design documents.

## Codec: pinned wire format and checked boundaries

Wire version 1 fixes both the private schema (`wire.rs`; the public core
types deliberately carry no serialization derives) and the rkyv archive
configuration: **rkyv `=0.8.18`, default features off, `std` + `bytecheck` +
`little_endian` + `pointer_width_32` + `unaligned`**. Little-endian archives
with 32-bit relative pointers at unaligned offsets are architecture-portable;
the exact pin exists because any change to the archive implementation or
format configuration changes the bytes, so an incompatible format requires a
new wire version. Three version layers stay separate: the envelope wire
version, the manifest semantic version (validated by the foundation
manifest check), and each extension's `StateFormat` version.

Every image is one envelope:

```text
offset  size  field
0       8     magic "HORKYV\0\0"
8       4     u32 LE wire version (currently 1)
12      4     u32 LE reserved format flags (must be zero)
16      8     u64 LE archive length (must equal remaining bytes exactly)
24      …     rkyv archive of the private Image (Manifest | Extensions)
```

`encode_manifest` and `encode_extensions` (the latter encodes standalone
metadata without requiring a bundle) first walk the input against a budget —
one node per resource/bundle/member/extension, logical content bytes for
identifiers, generation, format names, inline payloads and range resource
names — then validate semantics (`manifest.validate()`, extension `validate()`
plus duplicate-key rejection) and serialize through a bounded writer that
fails with `LimitExceeded` rather than exceeding `max_bytes - 24`.

`decode` checks input size against `max_bytes`, verifies the envelope (magic,
supported version, zero reserved flags, exact length — trailing bytes are
rejected), bytechecks the archive via `rkyv::access`, **then traverses the
validated archived data against the same node/content budget before any owned
allocation**, and only then deserializes. Afterward the public types are
rebuilt and re-validated end to end: identifiers are re-created through their
checked constructors (`ResourceId` failures surface as `InvalidResourceId`,
others as `Manifest` errors), extension keys are re-checked for duplicates,
overflowing ranges surface as `State` errors, and the reconstructed manifest
must pass `manifest.validate()` again. Decoding a manifest image as extensions
(or vice versa) is `WrongImageKind`, not an error hidden by reinterpretation.

Two honest boundaries from the crate's own contract:

- **bytecheck validates structure, not trust.** There is no checksum or
  authentication; a mutation can produce different but structurally valid
  metadata (the fuzzing test says exactly this). Integrity or origin
  guarantees belong to a wrapping layer, not to this codec.
- **Limits bound work, not exact peak allocator usage.** `Limits` (defaults
  16 MiB bytes / 65,536 nodes / 16 MiB content; ceilings 64 MiB / 1,000,000
  nodes) bound the input envelope, node count and logical content, checked
  before deserialization. Encoding caps its archive writer, but wire-type
  conversion and serialization scratch still need memory. Applications should
  choose limits for their trust boundary. A focused review additionally
  observed that bytecheck's own validation work is bounded by the envelope's
  byte limit, not by the tighter node budget, which governs the
  pre-allocation traversal and owned decode.

## Carrier: mandatory seals, positional I/O, size cap

`handle-off-state-memfd` is Linux-only (`cfg(target_os = "linux")`; an empty
crate elsewhere) and knows nothing about any codec. Its contract:

- **Creation** (`OwnedImage::create(bytes, max_bytes)`): validates
  `bytes ≤ max_bytes ≤ MAX_BYTES` (64 MiB shared API ceiling), creates a
  memfd with `CLOEXEC | ALLOW_SEALING`, writes positionally with an `EINTR`
  retry loop (`WriteZero` on short writes), then applies **all four seals —
  `SEAL | WRITE | SHRINK | GROW`** — and re-validates through the adoption
  path.
- **Adoption** (`OwnedImage::adopt(OwnedFd, max_bytes)`): consumes the
  supplied owner — including on validation failure, so a rejected image
  closes rather than leaks — requires the full seal set, requires a regular
  file whose length fits the limit, and sets `CLOEXEC` explicitly (adopting
  a descriptor that lacked it is a test case). It never accepts raw
  descriptor numbers, only `OwnedFd`.
- **Borrowing** (`BorrowedImage::from_borrowed`): the same validation with no
  descriptor or status-flag changes at all.
- **Reads**: `read_all`/`read_range` use `pread` at explicit offsets only —
  duplicated descriptors' shared seek positions are never disturbed (a test
  pins a seek position and verifies it survives carrier reads). Ranges are
  bounds-checked against the validated immutable length; short reads are
  errors, `EINTR` is retried. `try_clone_to_owned` duplicates the descriptor
  with the usual shared-offset duplication semantics.

Why all four seals: without `WRITE`, writable mappings made before sealing
can persist (the test comments that size seals alone do not exclude existing
writable mappings), so `GROW|SHRINK|SEAL` without `WRITE` is rejected at
adoption. Seals are **immutability, not authentication or application
compatibility** — they say the image can no longer change, not who created it
or whether any application can interpret it. The crate is
`#![forbid(unsafe_code)]` and exposes no mmap interface; reads return owned
bytes and borrowing lends the handle.

## Independence and composition

The two crates are independently usable: the codec's registry dependency is
rkyv alone (plus workspace core crates for the validated types); the
carrier's normal registry dependency is rustix (`std` + `fs`) alone. Every
handle-off edge besides those in `state/memfd`'s lock entry is a
**dev-dependency** used by the composition test, not a build coupling.

That composition test (`state/memfd/tests/composition.rs`) sends one datagram
carrying a sealed metadata image (a wire-v1 encoded manifest) plus two
independent sealed state images, using the E2.1 transport; the receiver
adopts the metadata image (seals and length re-validated), decodes the
manifest, inserts the state descriptors into an `OwnedHandles` table keyed by
`ResourceId`, passes `validate_handles` (exact inventory), and reads each
bundle's `ResourceRange` positionally from the right image. The fixture
supplies its descriptor-order convention in the payload explicitly — the test
says so itself — and is **not** a delivery protocol; association, framing,
acknowledgment and rendezvous remain E2.3+ work.

## Dependency cost and facade isolation

- rkyv is the workspace's second registry family and its first exact-pinned
  dependency (`=0.8.18`). `Cargo.lock` grows from 7 to 42 registry entries
  (53 packages total with the 11 workspace crates): the rkyv runtime stack
  (bytecheck + derive, bytes, hashbrown, indexmap + equivalent, munge +
  macro, ptr_meta + derive, rancor, rend, simdutf8, tinyvec), the proc-macro
  chain (rkyv_derive, proc-macro2, quote, syn) which is build-time only, and
  wasm-gated chains (uuid → js-sys → wasm-bindgen, futures-\*, slab, bumpalo,
  cfg-if, once_cell, rustversion) that the lock records for other targets.
  The host-compiled set is bounded by the pinned feature selection; this
  checkpoint derived the closure from the committed manifests and lock
  rather than invoking cargo (see Verification).
- Facade isolation is preserved and was verified statically from the
  committed manifests: `state-rkyv` and `state-memfd` are separate opt-in
  features reexporting `state_rkyv`/`state_memfd`; `local` remains exactly
  `["invocation", "stdio"]`, so no feature path can pull rkyv or rustix into
  a local-only consumer; the foundation crates remain dependency-free.

## Verification

Toolchain: the repo-pinned mise-selected `nightly-2026-09-30` (rustc
1.101.0-nightly `5c543b0b8`, 2026-09-29); no stable/MSRV checks, per the
recorded policy.

Attributed to the implementing Sol session
`opencode://ses_ec717edaeffeP56i2EfWqe8r3v` (reported by it; not re-run by
this checkpoint):

- Workspace test run: **27 passing** — the 19 from the foundation and
  transport slices plus 5 codec unit tests, 2 carrier tests, and the 1
  composition test. The arithmetic is corroborated statically here (5
  `#[test]` in `state/rkyv/src/lib.rs`, 2 in `carrier.rs`, 1 in
  `composition.rs`).
- clippy with warnings denied, formatting, doc, and feature checks passed on
  the pinned nightly.

This checkpoint's own verification: read `0c1c9a90` end to end against the
maintained claims; derived dependency isolation and cost from the committed
manifests and lock **without invoking cargo** (concurrent sessions share this
working copy, and manifest-loading cargo commands were transiently broken
during the previous checkpoint by mid-edit workspace members); verified README
link targets. Codec coverage observed in tests: unaligned-input round trip,
every truncation length, unsupported/reserved-field/trailing-byte/corrupt
envelopes, byte-valid-but-semantically-invalid archives, budget enforcement
on both sides, and single-byte mutations of a valid archive never requiring
unchecked access.

## Acceptance accounting: pfd-handle-off-direct-state-carriers

Acceptance criteria (architecture E2.2): "Separately usable codec and
carrier; version/size validation and sealed-image round trip; multiple state
images associated with members."

- **Separately usable codec and carrier** — the codec depends on rkyv plus
  workspace core types only; the carrier on rustix `std`+`fs` only; they meet
  exclusively in dev-dependency tests; neither is in the facade's `local`
  feature.
- **Version/size validation** — envelope wire version with
  `UnsupportedVersion`, zero-reserved-flags and exact-length checks, manifest
  semantic-version validation on both encode and decode, byte/content/node
  budgets on both sides, and the carrier's 64 MiB ceiling with per-call
  `max_bytes` validation at creation and adoption.
- **Sealed-image round trip** — create seals `SEAL|WRITE|SHRINK|GROW` and the
  composition test restores an adopted, transferred, decoded image equal to
  the original, with mutation attempts (`pwrite`, shrink, grow) failing in
  the carrier tests.
- **Multiple state images associated with members** — the composition
  fixture carries two independent state images, each referenced by its own
  bundle member through a `ResourceRange` extension, restored through the
  handle table and read positionally.

Ticket status at documentation time: `in_progress`, with the tracker
recording this documentation checkpoint as the pending item. A focused
independent review (`opencode://ses_ec6f1618effeapZzCE6xoMvdTD`, recorded on
the ticket) has since found no substantive blocking issues, verifying
envelope bounds before archive access, budget checks before owned decode,
strict framing/semantic validation, and immutable seals with positional I/O;
it restates that application compatibility/authentication and actual carrier
associations remain caller responsibilities. (For context, the transport
ticket `pfd-handle-off-direct-rights` has also closed with its focused review
finding no substantive issues.)

## Links

- Direction and remaining capabilities:
  [architecture](/design/fd-handoff/architecture0.gpt6a.md)
- Coordination and toolchain policy:
  [work map](/design/fd-handoff/work.md#blocking-descriptor-transport-underway)
- Foundation contracts the codec re-validates:
  [foundation report](/doc/foundation/implementation0.gpt61s.md)
- Transport primitive the composition reuses:
  [transport report](/doc/transport/rights0.glm53max.md)
- Maintained exposure: [README — state images](/README.md#state-images)
