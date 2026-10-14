---
type: Readme
title: handle-off
description: Resource handoff workspace, optional primitives, and their verification boundaries.
resource: /README.md
status: implemented
generated: { by: "model:openai/gpt-6.1-sol#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB"
    title: Prior workspace/status additions for notify, fdstore and rendezvous, preserved here
  - resource: "opencode://ses_ec6b8650fffeM5meMdn7oL2CZs"
    title: Completed custody/rendezvous exposure and optional inspection limits, then updated the bounded-parser correction evidence
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented the optional inspection adapter and reported pinned-nightly and read-only live checks
  - resource: /doc/systemd/inspection0.gpt61s.md
    title: Optional inspection API, bounds, observation limits and attributed verification
  - resource: /doc/systemd/custody0.glm53max.md
    title: Notify/fdstore implementation and corrected message-bound verification
  - resource: /rendezvous/local/src/lib.rs
    title: Sample resolver policy and namespace-length-prefixed filename convention
---

# handle-off

> Pass, retain, and recover access to open resources, together with the
> information needed to continue using them.

`handle-off` is a Rust toolkit for resource handoff: direct descriptor
passing, systemd fdstore custody, socket activation, and cooperative
state-carrying handover. Stdio is one bundle; established network connections
are another. Linux is the initial systemd/memfd integration target; portable
Unix mechanisms remain separately buildable, and no async runtime is
mandatory.

This workspace is a fresh start. The retired `clid` experiment (a CLI→daemon
execution hand-off prototype, itself renamed from `prefork`) is not a
compatibility constraint; its code lives in version history, with its code
review in [REVIEW-0.md](REVIEW-0.md) and prior planning in `.transcript/`.

## Status

The foundation provides independent resource, bundle, opaque state, invocation,
and stdio crates, plus raw Unix datagram descriptor transfer, a checked state
codec, a sealed Linux state-image carrier, classic socket-activation metadata
handling, a sample correlated bundle-delivery protocol, service notifications
with fdstore offers and optional read-only membership inspection, and endpoint
rendezvous vocabulary with a sample policy. Generation custody and cooperative
lifecycle coordination are not implemented yet. The delivery sequence is proposed in
[design/fd-handoff/architecture0.gpt6a.md](design/fd-handoff/architecture0.gpt6a.md),
with the reading map in
[design/fd-handoff/README.md](design/fd-handoff/README.md).

## Workspace

| Crate | Description |
| --- | --- |
| `handle-off` | Facade with empty defaults and opt-in reexports |
| `handle-off-handles` (`handles/core`) | Logical resource IDs and explicit owned/borrowed FD tables |
| `handle-off-bundle` (`handles/bundle`) | Multiple named bundles and complete manifest membership checks |
| `handle-off-state` (`state/core`) | Opaque versioned extensions with inline or resource-range payloads |
| `handle-off-stdio` (`handles/stdio`) | Explicit borrowed/owned stdin, stdout, stderr roles |
| `handle-off-invocation` (`invocation/core`) | Native lossless argv, environment, and cwd snapshots |
| `handle-off-unix-rights` (`handles/unix-rights`) | Blocking Unix datagram payload/FD transfer using rustix |
| `handle-off-state-rkyv` (`state/rkyv`) | Checked bounded rkyv wire codec for manifests and state metadata |
| `handle-off-state-memfd` (`state/memfd`) | Sealed Linux memfd byte images with positional reads |
| `handle-off-activation` (`systemd/activation`) | Classic LISTEN_* parsing and explicit inherited-FD claiming |
| `handle-off-transport-unix` (`transport/unix`) | Sample correlated bundle delivery and acceptance over connected datagrams |
| `handle-off-systemd-notify` (`systemd/notify`) | Runtime-free notifications and bounded ordering barriers to explicit sockets |
| `handle-off-systemd-fdstore` (`systemd/fdstore`) | Named fdstore offers/removes with borrowed receipts and explicit verification |
| `handle-off-systemd-inspect` (`systemd/inspect`) | Optional Linux busctl service snapshots and kernel-metadata membership checks |
| `handle-off-rendezvous` (`rendezvous/core`) | Endpoint-resolution contract yielding unverified candidates |
| `handle-off-rendezvous-local` (`rendezvous/local`) | Sample explicit/environment/XDG/local resolution policy |

Foundation/local crates have **no external dependencies**. Facade features `handles`, `bundle`,
`state`, `stdio`, and `invocation` enable individual reexports; `local` enables
only `stdio` and `invocation`. Descriptor APIs require Unix. Native invocation
types and metadata can be used without descriptor operations.
The separate `unix-rights` feature adds rustix; `state-rkyv` adds exact-pinned
rkyv and `state-memfd` adds rustix on Linux; `activation` adds rustix on Unix;
`systemd-notify` and `systemd-fdstore` compose workspace crates plus rustix;
`systemd-inspect` separately adds serde/serde_json/roxmltree, reuses rustix on Linux,
and requires an external trusted `busctl` executable at query time;
`transport-unix` and the std-only `rendezvous` crates compose workspace crates
only. None are included in `local`.

## Local use

```toml
[dependencies]
handle-off = { version = "0.1.0", features = ["local"], default-features = false }
```

This is the intended dependency declaration; packages have not been published.
Use workspace path dependencies for now. The example handler receives invocation
and stdio explicitly, copies stdin bytes to stdout, and writes a summary to stderr:

```sh
cargo run -p handle-off-local-example -- arbitrary arguments < input > output
cargo nextest run --workspace --all-features
```

Arguments include argv[0]; Unix non-UTF-8 arguments, environment, and cwd remain
native bytes. Invocation capture does not execute a command or change process
globals. Do not mix buffered standard-I/O access with direct descriptor I/O
unless the application coordinates buffering.

Resource IDs express caller-declared associations, not FD-number/OFD identity.
Manifest validation checks structure and complete membership; applications still
validate extension compatibility, resource kinds, carrier bounds, and the right
to resume I/O. Descriptor duplication preserves shared offsets/status flags and
does not revoke the original owner's access.

See the [foundation implementation report](doc/foundation/implementation0.gpt61s.md)
for APIs, extraction ownership, checks, and the measured local-build baseline.

## Raw descriptor transfer

`handle-off-unix-rights` sends a nonempty payload plus borrowed FDs as one Unix
datagram and receives owned FDs with explicit size/count limits. Sender ownership
is retained. Receive rejects payload/control truncation and closes acquired
references before returning an error. No stream framing or application adoption
permission is implied; successful send is not an acknowledgment.

The independent two-process example transfers a file reference. Use a private
directory and run receive/send in separate terminals:

```sh
umask 077
mkdir -p .test-agent/rights-demo
cargo run -p handle-off-raw-rights-example -- receive .test-agent/rights-demo/socket
cargo run -p handle-off-raw-rights-example -- send .test-agent/rights-demo/socket README.md
```

The receiver removes its socket path after receipt; it never removes a preexisting
path before binding. Failures can leave the path for deliberate cleanup. The raw
primitive does not authenticate peers; applications own endpoint permissions and
adoption policy. Received FDs have CLOEXEC; atomicity of that setting varies by
Unix platform. See the [rights checkpoint](doc/transport/rights0.glm53max.md).

## State images

`handle-off-state-rkyv` encodes manifests or standalone opaque state metadata
into a versioned envelope — wire version 1, pinned rkyv 0.8.18 configured for
little-endian, 32-bit relative pointers, unaligned archives. Encoding is
byte-bounded; decoding validates the envelope, bytechecks the archive within
node/content budgets, then repeats full semantic validation on the rebuilt
types. Limits bound input and logical content, not exact peak allocator
memory; choose them for your trust boundary. There is no checksum or
authentication: bytecheck validates structure, not trust.

`handle-off-state-memfd` creates immutable sealed Linux memfd images — the
full WRITE/GROW/SHRINK/SEAL seal set is mandatory and re-verified at adoption,
with a 64 MiB ceiling — and reads them positionally, leaving duplicated
descriptors' shared offsets untouched. The carrier depends on no codec and the
codec on no carrier; seals are immutability, not authentication or application
compatibility.

A composition test sends a sealed manifest image plus two independent state
images in one datagram and restores all of them; its descriptor ordering is an
explicit fixture convention, not a delivery protocol. See the
[state checkpoint](doc/state/carriers0.glm53max.md).

## Socket activation

`handle-off-activation` parses classic `LISTEN_PID`/`LISTEN_FDS`/
`LISTEN_FDNAMES` snapshots — read-only capture, no environment mutation —
and separates pure validation (foreign PIDs ignored before their count or
names are inspected; strict PID/count/tag rules with per-count bounds) from
claiming the inherited descriptors, which is an explicit `unsafe` operation:
the caller must establish exclusive, valid, otherwise-unowned slots that
neither parsing nor `fcntl` probing can prove. Claiming takes all owners,
sets CLOEXEC preserving other flags, and closes the entire batch on failure.
Tags are nonunique manager labels; `take_unique` errors leave every owner in
the batch. The optional newer `LISTEN_PIDFDID` process-identity check is not
implemented.

The example's tests use a synthetic single-threaded manager that installs
real TCP listeners on inherited FD 3/4 and execs the consumer — real exec
boundary and sockets, but **not** live systemd: no manager, socket unit,
sd_notify, or fdstore participates. The crate stays runtime-free with rustix
as its only dependency. A focused review's P2 names-parser finding (segment
collection before the count check) is fixed and regression-covered. See the
[activation checkpoint](doc/systemd/activation0.glm53max.md).

## Bundle delivery

`handle-off-transport-unix` is an optional sample backend, not a universal
transport: each side supplies a fresh, already-connected datagram socket it
uses exclusively, with serialized single-flight exchanges and no discovery,
authentication, or runtime policy. One offer carries a wire-v1 manifest
plus descriptors — ancillary slot `i` is exactly `manifest.resources[i]`,
a transient delivery binding, not FD identity. ACK/NACK frames echo the
offer ID and generation; an explicit accept or reject is the receiver's
deliberate decision, and acceptance means custody only — never command
completion, lifecycle commit, authentication, or revocation of the sender's
originals.

The protocol installs no timeouts (configure the socket first). Any failure
after an offer submission attempt is conservatively indeterminate: the
endpoint poisons and never auto-retries, because the datagram may still be
queued at the peer and resending could deliver the same rights twice. A
failed ACK returns the accepted delivery to the application for an explicit
decision. Limits (defaults 16 KiB / 64 FDs / 4096 nodes; ceilings 64 KiB /
253 / 1M) bound frame, manifest, and ancillary work together. The crate
depends only on workspace crates — no memfd, no runtime — and the
[direct-bundles example](examples/direct-bundles/src/main.rs) transfers two
sealed-memfd bundles between processes, rejecting extensions at scopes it
does not interpret before accepting. See the
[bundle delivery checkpoint](doc/transport/bundles0.glm53max.md).

## Manager custody

`handle-off-systemd-notify` sends `READY`, single-line `STATUS`, `STOPPING`,
and individual `WATCHDOG` notifications to explicit filesystem/Linux abstract
endpoints or a read-only `NOTIFY_SOCKET` snapshot. It installs no timers and
never mutates the environment. Messages are at most **4096 bytes**, including
assignment prefixes. `NoManager` means no endpoint; `Submitted` means kernel
enqueue, not manager processing or acceptance.

The bounded `BARRIER` pipe-closure protocol orders earlier notifications only
when manager support is known. It does not establish fdstore custody.
`handle-off-systemd-fdstore` offers borrowed FDs under nonunique names, removes
all entries under a name, and exposes explicit `FDPOLL` policy. Originals stay
caller-owned. Capabilities, capacity snapshots, and receipt verification are
explicit application inputs; the primitives include no manager inspector or
automatic pruning. See the [custody checkpoint](doc/systemd/custody0.glm53max.md).

The separate `systemd-inspect` feature selects a read-only Linux `busctl`
adapter with explicit user/system bus, canonical service name and expected
MainPID. Snapshots bracket queries with service PID/InvocationID checks.
Offer verification compares all entries under the offered name with retained
originals' kernel metadata; **it does not prove OFD identity, access flags or
alias count**, pin an incarnation across lifecycle steps, or prove that closing
all originals is safe.
Removal verification observes absence only. Default bounds are a 3-second
aggregate query deadline, 1 MiB combined stdout/stderr per command, and 16,384
entries, enforced during typed dump decoding before excess-row fields are read;
timeout/output failures kill the command's process group and reap its
direct child. Capacity, unsupported inspection and permission/command errors
remain distinct. No Rust D-Bus library, async runtime or live state mutation
is required by the adapter. See the [inspection checkpoint](doc/systemd/inspection0.gpt61s.md).

## Endpoint rendezvous

`handle-off-rendezvous` returns **unverified candidates**, not live servers.
The std-only `handle-off-rendezvous-local` sample consults caller-configured
explicit, environment, XDG runtime, and local-directory sources in order,
stopping at the first yielding source. It never connects, probes, creates a
directory, or launches a server; invalid present configuration is an error,
not a reason to silently fall back.

Derived names use `<base>/<namespace-bytes>-<namespace>-<name>.sock`, where the
prefix is the namespace's UTF-8 byte length, avoiding ambiguous hyphen splits.
Already-connected transports bypass resolution. Neither rendezvous feature
enters `local`. See the [rendezvous checkpoint](doc/rendezvous/implementation0.glm53max.md)
and the [current sample policy](rendezvous/local/src/lib.rs).
