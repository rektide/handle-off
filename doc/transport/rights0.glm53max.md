---
type: ImplementationReport
title: Raw Unix datagram rights transfer
description: Checkpoint record of the Sol-implemented blocking SCM_RIGHTS primitive covering API and transport limits, borrowed-send/owned-receive ownership, CLOEXEC policy, truncation cleanup, rustix dependency cost, and pfd-handle-off-direct-rights acceptance accounting.
resource: /doc/transport/rights0.glm53max.md
status: implemented
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB"
    title: Documentation checkpoint verifying the transport slice against its code and completing the README exposure
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented the blocking Unix datagram rights primitive in 20c4edab and reported the workspace checks recorded below
  - resource: /handles/unix-rights/src/lib.rs
    title: API contract, limits, CLOEXEC policy and truncation cleanup
  - resource: /handles/unix-rights/tests/rights.rs
    title: Ownership, truncation, padding-bypass and socket-type coverage
  - resource: /examples/raw-rights/src/main.rs
    title: Two-process custody example semantics
  - resource: /examples/raw-rights/tests/processes.rs
    title: Independent-process file transfer test
  - resource: /handles/unix-rights/Cargo.toml
    title: rustix selection, bounded features and the time-feature rationale
  - resource: /Cargo.toml
    title: Workspace membership for the rights crate and example
  - resource: /handle-off/Cargo.toml
    title: Facade feature wiring isolating unix-rights from local
  - resource: /Cargo.lock
    title: Registry closure introduced by the transport slice
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E2.1 acceptance criteria and datagram-only scope
  - resource: /design/fd-handoff/work.md
    title: Checkpoint coordination and pinned-toolchain policy
  - resource: /doc/foundation/implementation0.gpt61s.md
    title: Foundation baseline this slice builds on and report precedent
  - resource: /rust-toolchain.toml
    title: Repo-pinned mise-selected nightly-2026-09-30
  - resource: /README.md
    title: Maintained exposure whose pending checkpoint link this report completes
---

# Raw Unix datagram rights transfer

`handle-off-unix-rights`, committed in `20c4edab` by the Sol medium session
(`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`), is the workspace's first
descriptor-transport capability and its first external dependency. This
checkpoint report records the primitive as implemented: API and transport
limits, ownership and CLOEXEC policy, rejection cleanup, the cost of the
rustix dependency, and acceptance evidence for `pfd-handle-off-direct-rights`.
It was written by a separate documentation session
(`opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB`) that read the commit and verified
the maintained claims against the code; it did not re-run the implementation
checks and owns no code, tickets, or design documents.

The primitive is raw custody, not a protocol. It is the E2.1 slice of the
[direct-delivery sequence](/design/fd-handoff/architecture0.gpt6a.md); bundle
membership, acknowledgment, rendezvous, and manager custody remain later work.

## API and transport limitations

- Blocking `send`/`receive` over AF_UNIX **SOCK_DGRAM** only. The socket type
  is enforced at runtime through `getsockopt`/`getsockname` because std's safe
  `UnixDatagram::from(OwnedFd)` conversion can wrap a stream socket. `send`
  requires a connected peer; `receive` also accepts a bound, unconnected
  datagram socket.
- One datagram carries one nonempty payload (up to `MAX_PAYLOAD` = 1 MiB, an
  allocation ceiling, not a promise any kernel accepts that size) plus an
  ordered list of borrowed descriptors (up to `MAX_FDS` = 253, matching the
  Linux `SCM_MAX_FD` ceiling; other kernels may accept fewer and report a
  syscall error). `ReceiveLimits { max_payload, max_fds }` bound each receive.
- Descriptor/payload association is by list order only —
  `Message { payload, descriptors }` carries no persistent resource or member
  identity. Mapping onto bundle manifests is an application/adapter concern.
- Send success means kernel submission, not application receipt or acceptance;
  no acknowledgment, authentication, stream/seqpacket framing, or application
  I/O permission is implied. Peers are not authenticated; endpoint permissions
  and adoption policy stay with the application.
- `EINTR` and `WouldBlock`/timeout errors are surfaced with no hidden retries.
  A putative partial datagram send returns `WriteZero` with delivery marked
  uncertain rather than retrying — a retry could deliver the same rights twice.
- Payload bytes are opaque: validate application contents after receipt and
  drop the `Message` on rejection. Kernel-valid ancillary messages are parsed
  by rustix; non-rights ancillary metadata is ignored; arbitrary malformed
  userspace CMSG buffers are not an input API of this crate.
- Socket nonblocking settings and timeouts are respected; the API changes no
  socket flags.

## Ownership: borrowed send, owned receive

- `send(&UnixDatagram, &[u8], &[BorrowedFd<'_>])` sends references. Sender
  ownership is always retained — originals remain valid after a successful
  send, and on error ownership never left the caller.
- `receive(&UnixDatagram, ReceiveLimits)` returns owned references. Receiving
  is a custody operation, not adoption permission; the application decides
  whether and when resumed I/O is permitted.
- Kernel duplication semantics apply: received references share file offsets
  and open-status flags with the sender's originals, but not descriptor
  numbers or `FD_CLOEXEC` state.

## CLOEXEC policy

Received descriptors always have `FD_CLOEXEC` set before being returned.

- Linux, Android, FreeBSD, NetBSD, OpenBSD and DragonFly use atomic
  `MSG_CMSG_CLOEXEC` in `recvmsg`.
- Other Unix targets set it with `fcntl(F_SETFD)` after receipt, leaving a
  fork/exec race window; callers there must coordinate concurrent process
  creation.
- Only Linux (x86_64-unknown-linux-gnu) has been tested in this slice.

## Rejection cleanup: truncation and limits

`receive` consumes exactly one datagram and discards the entire message on
rejection, closing every acquired reference:

- `MSG_TRUNC` or `MSG_CTRUNC` → `InvalidData`; the not-yet-drained rustix
  receive buffer's `Drop` closes all received rights.
- Zero-length payload, or more descriptors than the limit — including the
  Linux CMSG-padding case where a one-FD allocation can hold two FDs without
  `CTRUNC` — → `InvalidData`; the drained `Vec<OwnedFd>` drops them closed.
- A discarded datagram is never retried.
- Pre-syscall validation failures (invalid limits, wrong socket type) close
  nothing, because nothing was acquired; caller descriptors stay untouched.

The tests prove cleanup without global descriptor-count assumptions: after a
rejected message carrying references to a unique socket, the socket's peer
observes EOF (no received copy survived), the sender's originals remain valid,
and a subsequent datagram is still receivable on the same socket pair.

## rustix: choice and cost

Choice: std's `UnixDatagram` exposes no stable SCM_RIGHTS/ancillary API, and
this primitive needs a CMSG buffer whose ownership guarantees that received
rights are closed on every rejection path. rustix 1.1.5's
`SendAncillaryBuffer`/`RecvAncillaryBuffer` provide exactly that — its buffer
`Drop` drains and closes every received rights entry, as the commit message's
"Use rustix ancillary ownership" records. The crate itself is
`#![forbid(unsafe_code)]`; all syscall `unsafe` lives in rustix.

Cost:

- The workspace's first registry dependency. Foundation and local crates
  remain dependency-free; the facade keeps `unix-rights` out of `local` as a
  separate feature reexporting `handle_off_unix_rights` as `unix_rights`.
- Manifest: `rustix = { version = "1.1.5", default-features = false,
  features = ["std", "net", "time"] }`. The `time` feature exists because
  rustix 1.1.5's net-only Linux build references its gated timespec module
  (recorded in the manifest comment); dev-dependencies add `pipe` for tests.
- Host closure (x86_64-unknown-linux-gnu, observed via `cargo tree` before
  any later workspace edits): rustix 1.1.5 → bitflags 2.13.2 +
  linux-raw-sys 0.12.1. No libc on this target.
- `Cargo.lock` at `20c4edab` records seven registry entries in total; errno
  0.3.14, libc 0.2.190, windows-sys 0.61.2 and windows-link 0.2.1 appear only
  for other-target builds (rustix's libc fallback backend on non-Linux Unix,
  and Windows).

## Two-process example

[examples/raw-rights](/examples/raw-rights/src/main.rs): `receive SOCKET`
binds its socket (never removing a preexisting path; failures can leave the
path for deliberate cleanup), prints `ready`, receives one file reference,
removes its socket path after receipt, and copies the received file to
stdout. `send SOCKET FILE` connects an unbound socket and sends one borrowed
reference. The integration test spawns both as genuinely separate processes,
transfers a file containing non-UTF-8 bytes, and checks exit statuses and
socket-path removal.

## Verification

Toolchain policy: the repository pins the user's mise-selected
`nightly-2026-09-30` (rustc 1.101.0-nightly `5c543b0b8`, 2026-09-29). This
checkpoint confirmed the active toolchain matches and ran no stable/MSRV
checks — operator policy recorded in the
[work map](/design/fd-handoff/work.md#foundation-review-complete); the
foundation-era stable 1.93.1 probe is historical evidence only.

Attributed to the implementing Sol session
`opencode://ses_ec717edaeffeP56i2EfWqe8r3v` (reported by it; not re-run by
this checkpoint):

- Workspace test run: 19 passing — the 11 foundation tests (see the
  [foundation report](/doc/foundation/implementation0.gpt61s.md)) plus 7
  unix-rights unit tests and 1 two-process integration test. The arithmetic
  is corroborated statically here: 7 `#[test]` functions in `rights.rs`,
  1 in `processes.rs`, 11 elsewhere in the workspace.
- clippy with warnings denied, formatting check, and feature builds passed.

This checkpoint's own verification (documentation-specific, pinned nightly
only): read commit `20c4edab` against the maintained README claims; inspected
the dependency closure offline; verified README link targets. No broad checks
were re-run.

Concurrent-work note: while this report was written, another session began
the E2.2 state-carriers slice in the same working copy (workspace members
`state/rkyv` and `state/memfd` declared, `state/memfd` not yet present,
transiently preventing cargo workspace manifest loading). That work is not
part of this slice; the dependency statements above describe the committed
`20c4edab` state.

## Acceptance accounting: pfd-handle-off-direct-rights

Acceptance criteria (architecture E2.1): "Blocking send/receive with explicit
ownership/CLOEXEC policy; real pipe/socket transfers; malformed/truncated
ancillary data closes all acquired FDs."

- **Blocking send/receive with explicit ownership/CLOEXEC policy** — blocking
  API; borrowed send retains ownership, receive returns owned descriptors;
  CLOEXEC guaranteed before return, atomic on the `MSG_CMSG_CLOEXEC`
  platforms and documented as non-atomic elsewhere.
- **Real pipe/socket transfers** — tests move a real pipe, one end of a
  connected `UnixStream` pair, and `/dev/null` file references in a single
  datagram, then push data through the received pipe and socket ends; the
  example test transfers a real file between two independent processes.
- **Malformed/truncated ancillary data closes all acquired FDs** — payload
  truncation, control truncation (both partial and kernel-discarded rights),
  CMSG-padding limit bypass, and empty-payload cases all reject with every
  acquired reference closed, proven by peer EOF on a unique socket.

Ticket status at documentation time: `in_progress`, with the tracker
recording a focused review of `20c4edab` underway. Tracker updates, review
outcome, and any closure are parent-owned; this report supplies the evidence
record. The ticket notes additionally defer consumption APIs until actually
needed and flag scope-specific duplicate-extension coverage as a small future
addition; successful `sendmsg` never implies ownership transfer.

## Links

- Direction and remaining capabilities:
  [architecture](/design/fd-handoff/architecture0.gpt6a.md)
- Coordination and toolchain policy:
  [work map](/design/fd-handoff/work.md#blocking-descriptor-transport-underway)
- Foundation this builds on:
  [foundation report](/doc/foundation/implementation0.gpt61s.md)
- Maintained exposure:
  [README — raw descriptor transfer](/README.md#raw-descriptor-transfer)
