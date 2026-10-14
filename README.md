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
codec, a sealed Linux state-image carrier, and classic socket-activation
metadata handling. Bundle delivery protocols, fdstore custody, and cooperative
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

Foundation/local crates have **no external dependencies**. Facade features `handles`, `bundle`,
`state`, `stdio`, and `invocation` enable individual reexports; `local` enables
only `stdio` and `invocation`. Descriptor APIs require Unix. Native invocation
types and metadata can be used without descriptor operations.
The separate `unix-rights` feature adds rustix; `state-rkyv` adds exact-pinned
rkyv and `state-memfd` adds rustix on Linux; `activation` adds rustix on Unix.
None are included in `local`.

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
