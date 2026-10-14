---
type: ImplementationReport
title: Classic socket-activation parsing and inherited descriptor ownership
description: Checkpoint record of the Sol-implemented E3.1 slice covering read-only LISTEN_* snapshot capture, pure metadata validation, the unsafe inherited-descriptor claim contract, duplicate-tag resolution, the synthetic-manager example, and pfd-handle-off-systemd-activation acceptance accounting.
resource: /doc/systemd/activation0.glm53max.md
status: implemented
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB"
    title: Documentation checkpoint verifying the activation slice against its code and extending the README exposure
  - resource: "opencode://ses_ec717edaeffeP56i2EfWqe8r3v"
    title: Implemented activation snapshot/parse/claim and the synthetic-manager example in 8274a60f and reported the workspace checks recorded below
  - resource: "opencode://ses_ec6f1618effeapZzCE6xoMvdTD"
    title: Focused review that found the names-parser allocation amplification and assessed ownership handling sound (findings read via the ticket record)
  - resource: /systemd/activation/src/lib.rs
    title: Snapshot capture, pure parse, unsafe claim contract and unit tests
  - resource: /systemd/activation/Cargo.toml
    title: Unix-gated rustix-only normal dependencies
  - resource: /examples/activation/src/main.rs
    title: Activation consumer with explicit safety rationale and socket-type checks
  - resource: /examples/activation/src/manager.rs
    title: Synthetic single-threaded launcher installing FD 3/4 before exec
  - resource: /examples/activation/tests/inherited.rs
    title: Inherited-slot integration tests across a real exec boundary
  - resource: /examples/activation/Cargo.toml
    title: Example-only libc/rustix dependencies and the second manager binary
  - resource: /Cargo.toml
    title: Workspace membership for the activation crate and example
  - resource: /handle-off/Cargo.toml
    title: Separate facade feature isolated from local
  - resource: /handle-off/src/lib.rs
    title: Facade reexport and updated facade crate docs
  - resource: /Cargo.lock
    title: Lock delta confined to rustix and example-only libc
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: E3.1 acceptance criteria
  - resource: /design/fd-handoff/work.md
    title: Checkpoint coordination and toolchain policy
  - resource: /rust-toolchain.toml
    title: Repo-pinned mise-selected nightly-2026-09-30
  - resource: /README.md
    title: Maintained exposure extended by this checkpoint
---

# Classic socket-activation parsing and inherited descriptor ownership

`handle-off-activation`, committed in `8274a60f` by the Sol medium session
(`opencode://ses_ec717edaeffeP56i2EfWqe8r3v`), delivers the E3.1 slice of the
[architecture](/design/fd-handoff/architecture0.gpt6a.md): explicit parsing of
classic PID-scoped socket-activation metadata and a separate, explicitly unsafe
operation that takes ownership of the inherited descriptors. A focused review
(`opencode://ses_ec6f1618effeapZzCE6xoMvdTD`) then found one P2 defect —
allocation amplification in the names parser — corrected in `19baae26` before
this documentation recorded the slice complete; see
[Correction after review](#correction-after-review). This checkpoint
report was written by a separate documentation session
(`opencode://ses_ec6fadd36ffe5ikLXsfbVLP6VB`) that read the commits, landed
that correction, and verified the maintained claims against the code; the
implementation's own workspace checks are attributed below, not re-run
beyond the targeted activation checks after the correction.

The crate implements **classic `LISTEN_PID`/`LISTEN_FDS`/`LISTEN_FDNAMES`**,
not the newer optional `LISTEN_PIDFDID` process-identity check. It is
independent of fdstore, bundle schemas, codecs, and any async runtime; its
Unix-gated normal dependency is rustix (`std` features only).

## Snapshot capture: read-only

`Snapshot::capture()` copies the three `LISTEN_*` values out of the
environment via `var_os` and nothing else. There is **no environment
mutation**: no removal, no rewrite, no descriptor ownership. Repeated capture
does not consume metadata. The docs are explicit that this convenience is not
a coordination mechanism: callers wanting a coherent snapshot should capture
at coordinated early startup; later parsing/claiming is not made safe by it;
filtering a child's environment, or explicitly coordinated edition-2024
`unsafe` removal, remains the application owner's job. Non-UTF-8 metadata is
rejected by parsing (the test constructs a raw `0xff` name value) — without
touching the environment.

## Pure parse: validation without side effects

`Snapshot::parse(current_pid, limits)` is a pure function over the captured
strings. Its decision structure:

- **No activation**: `LISTEN_PID` and `LISTEN_FDS` both absent; or a valid
  foreign PID (≠ current); or a zero count (with names, when present, fitting
  zero — an empty string is fine, any tag is a mismatch).
- **Error**: anything partial or malformed *for this process* — a present PID
  with missing count or vice versa (`MissingPid`/`MissingCount`), or any of
  the malformed cases below.
- A **foreign valid PID is ignored before its count/names are inspected**: a
  leaked parent environment containing `"41", "garbage", "bad\n"` parses as
  *no activation*, not an error. Only metadata that claims this process is
  held to full validation.

Validation details, each a distinct `ParseError`:

- **PID** — nonempty, ASCII digits only (`"+42"`, signs, whitespace, and
  non-UTF-8 values are `InvalidPid`), parses as `u32` (overflow:
  `InvalidPid`), and nonzero.
- **Count** — nonempty ASCII digits (`InvalidCount`), parses as `u64`
  (`CountOverflow` for 2⁶⁴), fits the inherited range so that
  `FIRST_FD + index` stays within `i32` (`DescriptorRangeOverflow`), and is
  within `limits.max_fds` (`TooManyDescriptors`). `Limits` defaults to 4096
  descriptors with a hard `MAX_FDS` ceiling of 65,536 (`InvalidLimits`
  above it); `MAX_NAME_BYTES` is 255.
- **Names** — must be UTF-8 (`InvalidNames`), pass a total-length pre-check
  of `count × 256` bytes before splitting, contain exactly one
  colon-separated tag per slot (`NameCountMismatch`), and each tag must be
  ASCII, control-free, and at most 255 bytes (`InvalidNames`). Segment
  collection is bounded to `count + 1` entries before the count check (see
  [Correction after review](#correction-after-review)), so an over-long
  colon run is rejected without materializing a segment per separator.

Names are **manager tags, not identities**: empty tags are retained (`Some("")`
— deliberately distinct from absent metadata, which stays `None` rather than
inventing a tag like `"unknown"`), repeated tags are retained, and reserved
tags such as `"stored"` receive no special meaning. `DescriptorInfo` exposes
`index`, `number` (`FIRST_FD + index`, delivery placement only — neither is
stable application identity), and the optional tag.

## Unsafe claim: preconditions and error ownership

`Metadata::claim()` is the boundary between pure metadata and real
descriptors, and it is `unsafe` because neither environment strings nor
`fcntl` probing can establish its preconditions. The documented contract:
every listed number must be a valid open descriptor in **this** process; the
caller transfers exclusive Rust ownership of all of them (no other
`OwnedFd`/`File`/socket wrapper may own these numbers); no concurrent
close/replacement/claim; claim at most once, normally directly after manager
exec and before threads or libraries can own the slots. Successful parsing or
`fcntl` probing does **not** prove these conditions. Callers must also
coordinate concurrent fork/exec until claim returns, because CLOEXEC is set
per descriptor with `fcntl`, not atomically for the batch. The crate is
`#![deny(unsafe_op_in_unsafe_fn)]` with the single unsafe conversion annotated.

Behavior: ownership of **all** listed descriptors is taken first
(`OwnedFd::from_raw_fd`), then CLOEXEC is set on each while preserving all
other descriptor flags. On any syscall failure the already-claimed owners
drop, **closing the entire batch** — the operation is all-or-nothing for
both ownership and CLOEXEC. Socket/file type and application suitability are
still the consumer's job; the example demonstrates the intended check
(`socket_type == STREAM` and `socket_acceptconn`) before converting to a
`TcpListener`.

## Named selection: duplicates and retention

`Activation` holds the claimed batch. `named(tag)` iterates every match —
tags are nonunique, so several descriptors may share one. `take_unique(tag)`
removes and returns the single match, or errors with `Missing(tag)` /
`Ambiguous(tag)`; **on either error every owner stays in the batch**. The
unit test proves retention without descriptor-count assumptions: after both
error kinds the batch length is unchanged, and once the batch is dropped, the
peers of both `"duplicate"` sockets observe EOF while the taken `"unique"`
socket's peer sees `WouldBlock` until the taken owner is dropped.

## Example: what the tests actually exercise

The example package has two binaries. The **synthetic manager**
(`manager.rs`) is a single-threaded fixture: it binds two real loopback TCP
listeners, duplicates them to high numbers, `dup2`s them onto FD 3 and 4
(the only `libc` use, confined to this fixture), prepares `LISTEN_*`
according to a mode (`valid`, `duplicate`, `absent`, `foreign`), removes
`LISTEN_PIDFDID` so a stale value cannot confuse the consumer, and **execs**
the consumer (preserving the PID that `LISTEN_PID` names; environment
preparation happens in this already-separate process, not a fork/`pre_exec`
callback). The **consumer** (`main.rs`) captures, parses, unsafely claims
with an explicit per-deployment SAFETY rationale, verifies CLOEXEC, selects
`listener` uniquely, type-checks the socket, prints a ready line, and serves
one ping/pong exchange.

The two integration tests spawn the manager and drive the consumer through a
real `exec` boundary and a real TCP connection: the named listener at the
second inherited slot accepts (`count=2`, `cloexec=true`, address echoed,
ping→pong), and duplicate tags fail with `Ambiguous` while absent/foreign
metadata reports no activation.

**These are synthetic inherited-FD tests, not live systemd tests.** No
systemd manager, socket unit, `sd_notify`, or fdstore participates; the
fixture changes only its own descriptor table and execs. The tests verify
this crate's parse/claim/select behavior against real inherited sockets
across a real exec — they make no claim about a real manager's delivery
semantics, which is precisely why `LISTEN_PIDFDID`'s process-identity
guarantee remains future work rather than an implicit assumption.

## Dependency boundary

- `handle-off-activation`: normal dependency is rustix 1.1.5
  (`default-features = false`, `std`) behind `cfg(unix)` — no runtime, no
  codec, no bundle/fdstore crate.
- The example's `libc` (the manager's `dup2`) and extra rustix features are
  example-only; `Cargo.lock` gains **no new registry entries** for this
  slice (libc 0.2.190 was already recorded).
- Facade: `activation` is a separate opt-in feature reexporting
  `handle_off_activation as activation`; `local` remains exactly
  `["invocation", "stdio"]`. Verified statically from the committed
  manifests at `8274a60f` (the concurrent E2.3 transport slice may edit
  Cargo/facade files in the working copy; all comparisons here use the
  committed revision). The facade crate docs now enumerate every feature,
  closing the documentation gap noted in the state-carrier checkpoint.

## Correction after review

The focused review of `8274a60f`
(`opencode://ses_ec6f1618effeapZzCE6xoMvdTD`, findings recorded on the
ticket) found one P2 defect and assessed the rest of the slice sound,
including all ownership handling. Defect: `Snapshot::parse` collected every
colon-separated segment of `LISTEN_FDNAMES` **before** checking the segment
count. The total-length pre-check permits up to `count × 256` bytes, and a
colon-only value at that bound yields a segment per byte — with a count
near `MAX_FDS` (65,536), a ~16 MiB input could materialize more than 16
million `&str` entries (~256 MiB) before `NameCountMismatch` rejected it.

Correction (`19baae26`, this documentation session): segment iteration is
bounded to `count + 1` entries, preserving the exact count-mismatch
semantics — fewer, exactly equal, and more segments all produce the same
outcomes as before — with all tag validation unchanged and no unbounded
vectors. A regression test drives the over-count path with the maximal
pre-check-permitted input (768 bytes of colons against a count of 3, plus a
single 255-byte tag declared as four slots), asserting `NameCountMismatch`
without giant fixtures. The parse doc comment now states the bound.

## Verification

Toolchain: the repo-pinned mise-selected `nightly-2026-09-30`; no stable/MSRV
checks, per the recorded policy.

Attributed to the implementing Sol session
`opencode://ses_ec717edaeffeP56i2EfWqe8r3v` (reported by it; not re-run by
this checkpoint):

- Workspace test run: **34 passing** — the 27 from earlier slices plus 5
  activation unit tests (4 parse-level, 1 claim/selection-level) and 2
  example integration tests. The arithmetic is corroborated statically here.
- clippy with warnings denied, formatting, doc, and feature checks passed on
  the pinned nightly.

This checkpoint's own verification: read `8274a60f` and `19baae26` end to
end against the maintained claims; derived the dependency boundary from the
committed manifests and lock without invoking cargo beyond the targeted
checks below (concurrent sessions share this working copy); verified README
link targets. On the repo-pinned nightly, after landing the correction:
`cargo nextest run -p handle-off-activation` — 6 passing including the new
`segment_overcount_is_rejected_without_materializing_every_segment`;
`cargo nextest run -p handle-off-activation-example` — 2 passing;
`cargo clippy -p handle-off-activation --all-targets -- -D warnings` and
`cargo fmt --check -p handle-off-activation` clean. The implementation
session's 34-test baseline plus the correction's regression make the current
workspace expectation 35 activation-inclusive tests. Unit coverage observed:
absent/foreign/zero activation, nonunique and empty tags with absence kept
distinct, every malformed-input class as a distinct error, bounded over-count
rejection, non-UTF-8 names rejected without environment mutation, and
selection-error ownership retention proven by peer EOF.

## Acceptance accounting: pfd-handle-off-systemd-activation

Acceptance criteria (architecture E3.1): "Multi-FD/name capture, PID
validation, ownership and CLOEXEC handling; independent example accepts a
socket-unit listener without bundle/state dependencies."

- **Multi-FD/name capture** — `Snapshot::capture()` reads the three classic
  variables read-only; parsing yields per-slot `DescriptorInfo` with optional
  tags; counts above one are exercised (two inherited slots in tests and the
  example).
- **PID validation** — strict ASCII-digit nonzero `u32` for this process,
  foreign PIDs ignored before their count/names are inspected.
- **Ownership and CLOEXEC handling** — the unsafe claim documents exclusive
  transfer preconditions, takes all owners, sets CLOEXEC preserving other
  flags, and closes the whole batch on failure; `take_unique` errors retain
  owners.
- **Independent example accepts a socket-unit listener without bundle/state
  dependencies** — the example accepts a *synthetic* socket-unit-style
  listener (a real listening TCP socket delivered via classic `LISTEN_*`
  across a real exec) with no bundle, state, codec, or fdstore dependency.
  It is not a live systemd socket unit; that distinction is the example's
  documented boundary.

Ticket status at documentation time: `in_progress`. The focused review has
completed with a single P2 finding — the names-parser allocation
amplification — corrected in `19baae26` by this session and covered by a
regression test; ownership handling was reviewed sound. Tracker updates and
closure remain parent-owned.

## Links

- Direction and remaining capabilities:
  [architecture](/design/fd-handoff/architecture0.gpt6a.md)
- Coordination and toolchain policy:
  [work map](/design/fd-handoff/work.md#blocking-descriptor-transport-underway)
- Transport primitive and state carriers this slice composes alongside:
  [transport report](/doc/transport/rights0.glm53max.md),
  [state report](/doc/state/carriers0.glm53max.md)
- Maintained exposure: [README — socket activation](/README.md#socket-activation)
