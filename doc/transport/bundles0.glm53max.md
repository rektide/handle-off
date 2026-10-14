---
type: ImplementationReport
title: Correlated Unix datagram bundle delivery and deliberate acceptance
description: Checkpoint record of the Sol-implemented E2.3 slice covering the single-flight connected-datagram endpoint contract, offer/receipt correlation by ID and generation, explicit accept/reject custody decisions, indeterminate-send poisoning without retry, transient FD-slot association, limits, and a measured raw-bundle example baseline.
resource: /doc/transport/bundles0.glm53max.md
status: implemented
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB"
    title: Documentation checkpoint verifying the bundle-delivery slice against its code, measuring the example baseline, and extending the README exposure
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented the correlated datagram bundle protocol and cross-process example in f6349a09 and reported the workspace checks recorded below
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: Focused review that found the example's unchecked generation/bundle-scope extensions (findings read via the ticket record)
  - resource: /transport/unix/src/lib.rs
    title: Endpoint contract, frame format, and acceptance-semantics documentation
  - resource: /transport/unix/src/unix.rs
    title: Protocol state machine, limits, errors, and seven unit tests
  - resource: /transport/unix/Cargo.toml
    title: Workspace-only dependencies with no memfd or runtime
  - resource: /examples/direct-bundles/src/main.rs
    title: Explicit-socket-path Linux receiver validating before acceptance
  - resource: /examples/direct-bundles/tests/processes.rs
    title: Multi-bundle sealed-image cross-process transfer with retained originals
  - resource: /examples/direct-bundles/Cargo.toml
    title: Example-only memfd dependency boundary
  - resource: /Cargo.toml
    title: Workspace membership for the transport crate and example
  - resource: /handle-off/Cargo.toml
    title: Separate facade feature isolated from local
  - resource: /handle-off/src/lib.rs
    title: Facade reexport and updated crate docs
  - resource: /Cargo.lock
    title: No new registry entries for this slice
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E2.3 acceptance criteria
  - resource: /design/fd-handoff/work.md
    title: Checkpoint coordination and toolchain policy
  - resource: /doc/foundation/implementation0.gpt61s.md
    title: Binary-baseline measurement precedent
  - resource: /doc/transport/rights0.glm53max.md
    title: Underlying datagram rights primitive
  - resource: /doc/state/carriers0.glm53max.md
    title: Manifest codec and sealed carrier this protocol composes
  - resource: /rust-toolchain.toml
    title: Repo-pinned mise-selected nightly-2026-09-30
  - resource: /README.md
    title: Maintained exposure extended by this checkpoint
---

# Correlated Unix datagram bundle delivery and deliberate acceptance

`handle-off-transport-unix`, committed in `f6349a09` by the Sol medium
session (`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`), delivers the E2.3
slice of the [architecture](/design/fd-handoff/architecture0.gpt6a.md): an
optional concrete sample backend that carries one checked manifest plus its
descriptors over a connected Unix datagram pair and makes receiver acceptance
an explicit, correlated decision. A focused review
(`opencode://ses_ec6f1618effeapZzCE6xoMvdTD`) found the protocol sound but
one P2 sample defect — the example accepted manifests with unchecked
generation/bundle-scope extensions — corrected in `2d131eff` by this session
before documentation recorded the slice complete; see
[Correction after review](#correction-after-review). This checkpoint
report was written by a separate documentation session
(`opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB`) that read the commits, landed
that correction, verified the maintained claims against the code, measured
the example baseline recorded below, and re-ran none of the implementation's
broad checks. It owns only documentation and the example correction.

## Endpoint contract: exclusive, fresh, connected, single-flight

`Endpoint::new` takes a **fresh, already-connected** `UnixDatagram`
(`peer_addr` is checked; an unconnected socket is an error) plus `Limits`.
The endpoint assumes exclusive protocol custody: no retained clones, no
interleaved raw reads/writes, and an empty receive queue at handoff.
Exchanges are serialized, half-duplex, and **single-in-flight** — one offer,
one correlated response; simultaneous offers from both peers are not
supported. There is no discovery, authentication, or runtime policy, and
rewrapping a used socket as a fresh endpoint resets correlation history, so
it must never be done. Timeouts: the protocol installs **none** — a missing
ACK on a default blocking endpoint can wait forever; the caller configures
timeouts or nonblocking mode on the socket *before* construction. Transport
size ceilings do not guarantee kernel send-buffer capacity on every Unix.

## V1 frame

Every datagram is: 8-byte magic `HOUDG\0\0\0`, LE `u16` version, `u8` kind
(offer=1, accepted=2, rejected=3), one zero reserved byte, LE `u64` offer
ID, LE `u16` generation byte length, LE `u16` FD count, LE `u32` body
length, then UTF-8 generation and body — a 28-byte header. The body of an
offer is exactly one wire-v1 rkyv manifest; ACK/NACK frames carry neither
body nor FD count (any count or body on them is `InvalidFrame`). Parsing
fails closed on short input, bad magic, unsupported version, nonzero
reserved bytes, unknown kinds, zero IDs, empty/oversized/control-character
generations, and length fields that do not sum exactly to the datagram
(trailing bytes rejected).

## Correlation: offer IDs and generations

Local offer IDs start at 1 and strictly increase (`IdExhausted` on `u64`
wraparound). ACK/NACK must echo **both** the offer ID and the generation; a
response matching neither/mismatching either is `UnrelatedResponse` and the
endpoint stays poisoned — an unrelated or duplicate acknowledgment never
confirms a delivery. On the receiving side, an offer ID less than or equal
to the last received ID is `DuplicateOffer`.

## FD-slot association is transient

For offers, **ancillary slot `i` is exactly `manifest.resources[i]`** —
nothing more. Resource order is a delivery binding, not FD/OFD identity:
nothing about slot position survives the exchange. Duplicate resource IDs
are invalid; aliasing uses repeated member references to the same resource
ID instead. The receiver checks the frame's FD count against the actual
received count and against `manifest.resources.len()` (both
`DescriptorCountMismatch`), zips resources with descriptors into an
`OwnedHandles` table, and requires `validate_handles` to pass — exact
inventory, independent of order.

## Sender: phases, indeterminacy, and poisoning

`send(manifest, handles)` returns `Result<Receipt, SendError>` where
`SendPhase` distinguishes:

- **BeforeSend** — preflight failures (poisoned endpoint, manifest/handle
  membership mismatch, limits, ID exhaustion, manifest encoding): nothing
  was submitted; the endpoint remains usable.
- **Indeterminate** — any failure after the offer submission was attempted
  (I/O errors, missing ACK within the caller's timeout, malformed or
  unrelated or rights-carrying responses): conservatively unknown. The
  submitted datagram may still be queued at the peer — the test proves a
  timed-out send left one deliverable descriptor queued — which is exactly
  why **no retry is automatic**: resubmitting could deliver the same rights
  twice. The endpoint is poisoned before submission and stays poisoned;
  replacement or recovery is caller-owned. Only a correlated NACK
  (`Rejected` phase) converts an outcome to a definite refusal, and only a
  correlated ACK yields a `Receipt` and unpoisons.
- **Rejected** — the peer's explicit, correlated refusal.

The sender keeps every original descriptor; a receipt never revokes them
(the example verifies retained originals still read after delivery).

## Receiver: validate, then decide explicitly

`receive()` consumes one datagram and performs all structural work — frame
validation, manifest decode with full semantic revalidation, exact FD
inventory — and emits **no ACK**. Malformed input drops all acquired owners
and poisons the endpoint. The returned `Offer` borrows the endpoint (the
borrow excludes concurrent operations) and holds the `Delivery` (manifest +
owned handles) pending an application decision:

- `accept()` sends the correlated ACK. If the ACK itself fails, the decision
  was made but is unconfirmed: `AcceptError` returns the `error` **and the
  entire owned delivery in a box** for an explicit application choice — the
  test reads live data through those retained handles after a failed ACK. A
  lost ACK cannot undo the receiver's acceptance.
- `reject()` drops the offer's inventory *before* submitting the correlated
  NACK. The application must separately retire any copies it made while
  validating; **rejection is not global revocation** of descriptor
  references.
- Dropping an `Offer` without deciding closes the acquired references
  without any ACK and leaves the endpoint poisoned.

**Acceptance is custody acceptance only** — not command completion, not
handover commit, not authentication, not revocation of the sender's
originals, and not permission for simultaneous I/O. Applications decide
whether resumed I/O is ever justified.

## Limits and defaults

`Limits { max_datagram, max_fds, max_nodes }` defaults 16 KiB / 64 / 4096;
ceilings `MAX_DATAGRAM` 64 KiB, FD count bounded by the rights primitive's
`MAX_FDS` (253), nodes by the codec's `MAX_NODES` (1,000,000); minimum
datagram 52 bytes (header + codec envelope). Generations are nonempty, at
most 256 bytes, control-free. The codec and receive limits are derived from
the same values, so one `Limits` bounds frame, manifest, and ancillary work
together; the ACK receive uses `max_fds: 0`, so an ACK carrying rights is an
error whose attached references are closed.

## Dependency boundary and example

The crate depends only on workspace crates — bundle, handles, state-rkyv,
unix-rights — **no memfd, no async runtime, no new registry entries** in
`Cargo.lock`. The facade exposes it as the separate `transport-unix`
feature; `local` remains exactly `["invocation", "stdio"]`. Sealed-image
use lives in the example: [direct-bundles](/examples/direct-bundles/src/main.rs)
is a Linux receiver with explicit socket paths (`RECEIVER LOCAL PEER`, peer
already bound — a sample, not a discovery policy) that sets a 5-second
read timeout, applies its extension policy — it interprets extensions only
on members (`example.bytes` v1 `ResourceRange`s) and rejects any
generation- or bundle-scope extension, required or optional, before
accepting — validates image contents/seals via `BorrowedImage` **before**
accepting custody, then accepts, removes only its own socket path, and
writes the images out. Its [cross-process test](/examples/direct-bundles/tests/processes.rs)
transfers **two bundles over two sealed memfd images between independent
processes** with manifest resource order deliberately different from the
handle table's sorted iteration, asserts the correlated receipt, the exact
concatenated bytes (including non-UTF-8 data), and that the sender's
retained originals still read after the receipt; a second regression test
covers the corrected extension policy (below).

## Correction after review

The focused review of `f6349a09`
(`opencode://ses_ec6f1618effeapZzCE6xoMvdTD`, findings recorded on the
ticket) found the protocol sound but one P2 defect confined to the sample
receiver: it validated member extensions only, then called `accept`, so
unknown **required** manifest-generation and bundle-scope extensions passed
unchecked into accepted custody.

Correction (`2d131eff`, this documentation session): the sample now rejects
**any** extension at generation or bundle scope — required or optional —
before acceptance. That is the explicit unknown/optional policy for this
narrowly defined sample: it gives those scopes no meaning, so silently
ignoring metadata it cannot interpret would be wrong here, while stricter
member-scope behavior (only `example.bytes` v1) was already in place. The
**generic transport remains fully opaque to extensions** — no protocol or
transport code changed; this strictness is the consuming application's
documented policy. The regression test offers, for each scope, a manifest
carrying a required extension in an unknown format and asserts both halves
of rejection-before-acceptance: the child exits with the scope-specific
error, and the sender never receives a correlated ACK
(`SendPhase::Indeterminate`), so no receipt — and therefore no acceptance —
ever existed.

## Verification and measured baseline

Toolchain: the repo-pinned mise-selected `nightly-2026-09-30`; no stable/MSRV
checks, per the recorded policy.

Attributed to the implementing Sol session
`opencode://ses_ec717edaeffeP56i2EfWqe8r3v` (reported by it; not re-run by
this checkpoint):

- Workspace test run: **43 passing** — the 35 from earlier slices plus 7
  transport unit tests and 1 cross-process example test. The arithmetic is
  corroborated statically here. Unit coverage observed: original-owner
  retention through accept and reject, fail-closed version/framing/
  generation/inventory cases with peer-EOF cleanup proofs, byte-valid but
  semantically corrupted archives, truncation cleanup, dropped offers never
  ACKing, duplicate offer IDs, unrelated/duplicate ACKs never confirming,
  preflight-vs-indeterminate phases with a live queued descriptor after
  timeout, failed-ACK delivery retention, and rights-carrying ACK cleanup.
- clippy with warnings denied, formatting, doc, and feature checks passed
  on the pinned nightly.

This checkpoint's own verification: read `f6349a09` and `2d131eff` end to
end against the maintained claims; confirmed dependency isolation from the
committed manifests and lock; on the repo-pinned nightly, after landing the
correction, ran targeted example checks — `cargo nextest run
-p handle-off-direct-bundles-example` (2 passing, including the extension
regression), `cargo clippy -p handle-off-direct-bundles-example
--all-targets -- -D warnings`, and `cargo fmt --check` — all clean. The
implementation session's 43-test baseline plus the correction's regression
make the current workspace expectation 44 bundle-inclusive tests. It also
measured, with the foundation report's method (default release
optimization, symbols stripped), the raw bundle example:
**472,096 bytes**, dynamically linked to libc and libgcc_s plus the loader,
with a host normal-dependency closure of the 8 workspace crates plus 20
registry crates (rkyv 0.8.18 with bytecheck/derive, hashbrown, munge/macro,
ptr_meta/macro, rancor, rend, simdutf8, rkyv_derive, rustix, bitflags,
linux-raw-sys, proc-macro2, quote, unicode-ident, and syn in two versions —
2.0.119 via bytecheck_derive and 3.0.9 via rkyv_derive). The remaining lock
entries (bytes, indexmap, equivalent, tinyvec, uuid and the wasm chains)
are optional-feature or other-target records that do not compile into this
build. This refines the lock-level enumeration in the
[state report](/doc/state/carriers0.glm53max.md) with a measured host
closure.

This is a **raw bundle example** baseline only. It is not the deliverable
of `pfd-handle-off-direct-build-baseline`, which compares a chosen remote
build for the thin CLI client whose actual result sample (E5) is not built
yet; that ticket remains open and is not closed by this measurement.

## Acceptance accounting: pfd-handle-off-direct-protocol

Acceptance criteria (architecture E2.3): "Select/document initial socket
type; exact metadata/FD association, limits, delivery acknowledgment,
disconnect semantics, reordered/missing membership and partial-delivery
cases tested."

- **Socket type selected and documented** — connected AF_UNIX `SOCK_DGRAM`,
  exclusive fresh endpoints, single-flight half-duplex exchanges, explicit
  no-discovery stance; V1 frame fully specified.
- **Exact metadata/FD association** — slot `i` equals `manifest.resources[i]`
  as a transient delivery binding; exact-inventory validation on receipt.
- **Limits** — one `Limits` value bounding frame, manifest, and ancillary
  work with ceilings and defaults documented above.
- **Delivery acknowledgment** — correlated ACK/NACK echoing ID and
  generation; explicit accept/reject; indeterminate sends poisoned without
  retry; a lost ACK cannot undo acceptance.
- **Disconnect semantics** — peer drop surfaces as indeterminate I/O failure
  with poisoning; recovery is caller-owned; dropped offers never ACK.
- **Reordered/missing membership and partial delivery tested** — reordered
  resource/handle-table ordering in the example and exact-inventory checks;
  missing membership fails preflight (`HandleMembership`); count mismatches
  and payload/ancillary truncation are tested with cleanup proofs.

Ticket status at documentation time: `in_progress`, deliberately kept open
by the parent until this correction and documentation completed. The
focused review found the protocol sound; its single P2 finding — the
example's unchecked generation/bundle-scope extensions — is corrected in
`2d131eff` with a before-acceptance regression. No protocol or transport
changes were made, and no broader protocol work is claimed.

## Links

- Direction and remaining capabilities:
  [architecture](/design/fd-handoff/architecture0.gpt6a.md)
- Coordination and toolchain policy:
  [work map](/design/fd-handoff/work.md#blocking-descriptor-transport-underway)
- Composed primitives: [rights](/doc/transport/rights0.glm53max.md),
  [state carriers](/doc/state/carriers0.glm53max.md),
  [activation](/doc/systemd/activation0.glm53max.md)
- Maintained exposure: [README — bundle delivery](/README.md#bundle-delivery)
