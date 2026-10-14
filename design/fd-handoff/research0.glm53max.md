---
type: Research
title: fdstore and descriptor handoff — grounded findings for the fresh clid successor scope
description: What opencode-bus-proxy implements today, kernel/systemd vocabulary, fdstore preservation semantics, restart-continuity feasibility for a CLI stdio server, and reusable mechanisms with unresolved choices.
resource: design/fd-handoff/research0.glm53max.md
status: draft
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - { resource: "opencode://ses_ec7dfb23affeJiKeLioxASvnzO", title: "fdstore/handoff investigation for the fresh clid scope (direction, reading, and writing of this document)" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/lib.rs", title: "Implemented bind/adopt/store-once listener policy" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/systemd.rs", title: "Hand-rolled sd_notify wire protocol incl. fdstore SCM_RIGHTS" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/systemd/opencode-bus-proxy.service", title: "Hardened unit preset with FileDescriptorStoreMax=1" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/systemd/opencode-bus-proxy.socket", title: "Socket-activation unit owning the listen address" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/doc/event-multiplexer/operations.md", title: "Maintained fdstore composition rules and restart-continuity claims" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/design/event-multiplexer/architecture.md", title: "Service-manager crate boundary and startup ordering" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/.beads/issues.jsonl", title: "ocbp-systemd* tickets: implemented closed vs open proposals" }
  - { resource: "file:///home/rektide/archive/systemd/systemd/man/sd_notify.xml", title: "FDSTORE/FDNAME/FDPOLL/FDSTOREREMOVE/BARRIER semantics (checkout 263~devel)" }
  - { resource: "file:///home/rektide/archive/systemd/systemd/man/sd_listen_fds.xml", title: "Activation handover protocol, fd 3+, FD_CLOEXEC, names" }
  - { resource: "file:///home/rektide/archive/systemd/systemd/man/systemd.service.xml", title: "FileDescriptorStoreMax=/FileDescriptorStorePreserve= semantics and versions" }
  - { resource: "file:///home/rektide/archive/systemd/systemd/docs/FILE_DESCRIPTOR_STORE.md", title: "Official fdstore overview: lifecycle, seamless-restart pattern, access control" }
  - { resource: "../../clid-context/src/lib.rs", title: "Current ExecutionContext (rkyv) being generalized" }
  - { resource: "../../clid-client/src/lib.rs", title: "Current tokio datagram+SCM_RIGHTS client" }
  - { resource: "../../clid-daemon/src/lib.rs", title: "Current receive loop and OwnedFd handling" }
  - { resource: "../../README.md", title: "clid architecture as of this research" }
---

# fdstore and descriptor handoff — research

Situation: clid (this repo) passes an invocation plus stdin/out/err over one
Unix datagram with SCM_RIGHTS to a warm tokio daemon
([README](../../README.md), [clid-client](../../clid-client/src/lib.rs)). The new
direction is a fresh scope and name: **arbitrary passable bundles of resources
with stdio as only one example**, using systemd's file descriptor store plus
our own handoff, with heavy decomposition, an ultra-small (ideally blocking,
non-async) client, and Varlink as a future primary backend interest. No
compatibility obligations; no implementation yet. This document grounds the
architecture conversation. Nothing here is a design decision.

## 1. What opencode-bus-proxy actually does with fdstore

All claims in this section are **implemented and tested** unless marked
otherwise. The relevant code is `crates/service-manager` (~650 lines incl.
tests), deliberately free of libsystemd — the sd_notify protocol is hand-rolled
in
[`systemd.rs`](file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/systemd.rs).

Implemented behavior:

- **Activation capture before threads**
  ([lib.rs `Manager::capture`](file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/lib.rs#L31),
  called from
  [`main.rs`](file:///home/rektide/src/opencode-bus-proxy/crates/daemon/src/main.rs#L24)
  while single-threaded, marked `unsafe` for environment access): reads
  `LISTEN_PID`/`LISTEN_FDS`, requires the PID to match this process (a foreign
  PID is "not our activation", not an error), requires **exactly one**
  descriptor, probes fd 3 with `fcntl(F_GETFD)` *before* taking ownership so a
  closed handover is a clean config error instead of an I/O-safety abort, takes
  `OwnedFd::from_raw_fd(3)`, then removes `LISTEN_FDS`/`LISTEN_PID`/`LISTEN_FDNAMES`
  from the environment.
- **One code path for adoption, two provenances**
  ([`acquire_listener`](file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/lib.rs#L61)):
  a socket-unit handover and a *replayed fdstore* both arrive as ordinary
  `LISTEN_FDS` activation, and are treated identically. Adoption validates the
  descriptor is a bound internet socket (via `local_addr()`; a `ListenFIFO=`
  handover gets an actionable error), sets non-blocking, and never re-stores.
  Otherwise the daemon binds itself and **offers the listener to fdstore
  best-effort**: `FDSTORE=1\nFDNAME=listener` with the descriptor attached via
  SCM_RIGHTS. Failure is logged at debug and never fatal — without a manager it
  is a no-op (absent/empty `NOTIFY_SOCKET` ⇒ success).
- **Store-once invariant**: only a self-bound listener is stored; an inherited
  one (socket unit or fdstore replay) is never re-stored, because the manager
  already retains it and duplicate store entries would accumulate across
  restarts. Enforced structurally by `inherited.take()`; covered by the test
  `self_bound_listener_is_stored_but_adopted_listener_is_not`.
- **Address-ownership rule**: an explicit `--listen` together with any
  `LISTEN_FDS` handover is rejected — "the socket unit's `ListenStream=` owns
  the bind address". This deliberately also catches the restart case where a
  replayed fdstore first appears and a unit still passes `--listen`.
- **Wire details** ([`send_to_manager`](file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/systemd.rs#L99)):
  one fresh `SOCK_DGRAM|SOCK_CLOEXEC` socket per datagram, closed after send;
  abstract-namespace notify sockets (`@` prefix → leading NUL, no filesystem)
  and filesystem paths both handled; single-fd SCM_RIGHTS control buffer sized
  with `CMSG_SPACE`; `READY=1`(+`STATUS=`)/`STOPPING=1`/`WATCHDOG=1` at half of
  `WATCHDOG_USEC`, with the heartbeat task owned by a scoped guard (`Drop`
  aborts).
- **Tests** build a *fake service manager* — a plain `UnixDatagram` bound to a
  temp path or abstract name — and assert the exact payload
  (`FDSTORE=1\nFDNAME=listener`) and that the SCM_RIGHTS-received descriptor
  names the same listening socket. This fake-manager pattern is itself a
  reusable mechanism (§6).
- **Unit presets**
  ([service](file:///home/rektide/src/opencode-bus-proxy/systemd/opencode-bus-proxy.service),
  [socket](file:///home/rektide/src/opencode-bus-proxy/systemd/opencode-bus-proxy.socket)):
  `Type=notify`, `Restart=on-failure`, `FileDescriptorStoreMax=1` (harmless
  under the socket unit), full sandbox block, plus the socket unit whose
  `ListenStream=` owns the address. The maintained composition rules live in
  [operations.md §Descriptor
  store](file:///home/rektide/src/opencode-bus-proxy/doc/event-multiplexer/operations.md#descriptor-store-restart-continuity-without-a-socket-unit).

Proposed / open (beads `ocbp-systemd*`, `.beads/issues.jsonl` in that repo):

- `ocbp-systemd-status-source` (open): make `STATUS=` track event-source
  state instead of the static listen address. Watchdog stays liveness-only.
- `ocbp-systemd` (open epic remainder): `ListenStream=` on Unix sockets,
  `Type=notify-reload` follow-ups.
- **Not present anywhere**: memfd state serialization, `FDSTOREREMOVE=1`,
  `BARRIER=1`, multi-descriptor stores, `LISTEN_FDNAMES` parsing. The store is
  exactly one named listener. That is the honest generalization gap for clid:
  `requested()` rejects any count ≠ 1, the FDNAME is the fixed string
  `listener`, and the unit pins `FileDescriptorStoreMax=1`. Every one of those
  codepoints is a widening decision, not a rewrite.

## 2. Terminology: file descriptor / open file description / handle

Kernel-precise meanings (Linux; consistent with the man pages read):

| Term | Meaning | Lifetime / ownership |
| --- | --- | --- |
| **file descriptor (fd)** | An entry in one process's fd table; a small integer. `fd 3` in one process is unrelated to `fd 3` in another. | Per-process. Renumbered at every transfer boundary. |
| **open file description (OFD)** | The kernel object an fd refers to: file offset, status flags (`O_NONBLOCK`, access mode). `dup()`, `fork()`, and SCM_RIGHTS all produce *new fds for the same OFD*. | Refcounted by the kernel; lives until the last fd referring to it closes. **This is what actually "passes."** |
| (the referred object) | Socket, tty, pipe, memfd, regular file… | Independent of any process; pinned by any OFD. |

What moves over SCM_RIGHTS or into fdstore is a *reference to the OFD*, not a
number; the receiver sees a fresh fd number. fdstore "stores a duplicate
([dup(2)](https://man7.org/linux/man-pages/man2/dup.2.html) sense)" per
[FILE_DESCRIPTOR_STORE.md](file:///home/rektide/archive/systemd/systemd/docs/FILE_DESCRIPTOR_STORE.md#L29) —
the manager and the service hold fds for one shared OFD. Consequences worth
designing around:

- **Equality is OFD-identity, not number.** systemd dedups stored fds "pointing
  to the same object" before passing them back. A toolkit receiving a bundle
  can hold the same tty in two slots and should be able to say so.
- **`FD_CLOEXEC` is per-fd, not per-OFD.** Activation fds arrive with CLOEXEC
  *cleared* (they must survive the manager's `execve`; systemd clears it in its
  exec path, and `sd_listen_fds(3)` re-sets it on the service side). bus-proxy's
  raw adopt (`OwnedFd::from_raw_fd(3)`) does **not** re-set it — a small
  adopted duty worth making explicit rather than implicit. Similarly, received
  SCM_RIGHTS fds lack CLOEXEC unless received with `MSG_CMSG_CLOEXEC` (bus-proxy's
  test receiver does; its sender side needs no such flag).
- **Systemd's own vocabulary is small**: fdstore *entries*; `FDNAME=` a
  **non-unique tag** (ASCII, no control chars or `:`, ≤255 chars; one name for
  all fds in a message; unnamed entries are called `stored`); at activation
  `LISTEN_FDNAMES` is a colon-separated list, with special names `unknown`,
  `stored`, `connection`.

Defensible API vocabulary for the new scope (recommendation, grounded in the
precedents above):

- `fd` / `OwnedFd` — the transferable kernel reference. Never pretend it is
  more than per-process.
- **open file description** — use in docs wherever two parties "share" or
  "dedup" something; "passing a descriptor" is fine colloquially but the OFD is
  the preserved thing.
- **slot** (name + claim) — the bundle-level address for one fd: a stable
  string plus a *claimed kind* ("this is a tty", "this is a read end") that the
  receiver can verify. Avoid reusing bare "name" for this because systemd's
  FDNAME is deliberately weaker (non-unique tag); if we adopt FDNAME-compatible
  encoding, reserve and reject systemd's three special names.
- **bundle** — the unit of transfer (manifest + fds together), replacing
  today's fixed `ExecutionContext` + `[0,1,2]` triple.
- **store / retention** — manager-side custody (fdstore or our own).
- **handoff** — clid's client→daemon transfer; **adoption** — receiving
  manager-passed fds regardless of provenance (bus-proxy precedent);
  **replay** — fdstore→next-instance delivery.
- **handle** — recommend *not* using in the public API. It collides with
  Windows `HANDLE`, libuv handles, and "handle" as capability, and every reader
  will import a different assumption. If kept, define it narrowly as the
  slot-level token above. This is a naming choice to settle with the user, not
  a settled rule.

## 3. systemd fdstore semantics: what restart preserves, what it does not

Grounded in the archived systemd checkout (`263~devel`):
[sd_notify(3)](file:///home/rektide/archive/systemd/systemd/man/sd_notify.xml),
[sd_listen_fds(3)](file:///home/rektide/archive/systemd/systemd/man/sd_listen_fds.xml),
[systemd.service(5)](file:///home/rektide/archive/systemd/systemd/man/systemd.service.xml),
[FILE_DESCRIPTOR_STORE.md](file:///home/rektide/archive/systemd/systemd/docs/FILE_DESCRIPTOR_STORE.md).

Protocol recap: `FDSTORE=1` (+ `FDNAME=`) sent to `NOTIFY_SOCKET` with fds
attached; requires `FileDescriptorStoreMax=` non-zero (v219; **defaults to 0**);
held fds are **replayed to the next start/restart through the ordinary
`LISTEN_FDS` activation handover**, after any socket-unit fds, in arbitrary
order, duplicates removed. Tools: `systemd-analyze fdstore`, `systemd-run -p
FileDescriptorStoreMax=N`.

Preserved across a restart (kernel objects, via the manager's duplicate):

- The OFD itself: file offset, status flags (`O_NONBLOCK`), access mode.
- Listening sockets: **the accept/dgram backlog queues keep filling while the
  service is down** — this is the entire connection-refused-elimination claim,
  and it holds for queued *connects* (TCP) and, by the same kernel mechanics,
  for queued datagrams on a `SOCK_DGRAM`/`SOCK_SEQPACKET` rendezvous socket.
- Established connections *if their fds are themselves stored*; socket options
  persist (same socket object).
- Pipe buffers with unread data; tty device state (termios — device state, not
  OFD state); regular-file fds with offset; memfd contents.
- `flock` locks (attached to the OFD). And: while the manager holds a store
  entry, **the peer never sees `POLLHUP` even if the service closes its own
  copy** — closing requires an explicit `FDSTOREREMOVE=1`.

Not preserved:

- **Anything in process memory** — threads, userspace buffers, mappings,
  timers, signals. The documented remedy is serializing state into a memfd and
  storing that too (either continuously or right before a *clean* restart;
  after a crash you get whatever the last store captured).
- **fd numbers** (replayed from 3) and per-fd `FD_CLOEXEC` state.
- **POSIX record locks** (`fcntl F_SETLK`): per-process, released at process
  exit — only `flock` survives.
- **Protocol state of live streams** unless the connection fds themselves were
  stored; bus-proxy explicitly does not store accepted connections — "existing
  streams are live-only and end with the process that served them"
  ([operations.md](file:///home/rektide/src/opencode-bus-proxy/doc/event-multiplexer/operations.md#L283)).
  Preserving the *listener* and preserving *sessions* are different tiers.
- Store-lifetime caveats: the store is **discarded when the service is stopped
  with no job queued** (default `FileDescriptorStorePreserve=restart`, v254:
  `no|restart|yes|on-success`); a restart job keeps it, a separate stop→start
  likely loses it. Pollable fds are **auto-dropped on `EPOLLHUP`/`EPOLLERR`**
  unless `FDPOLL=0` (v246) — which is usually what you want for dead peers and
  dangerous for fds that legitimately poll-error.
- Operational limits: **store success is not acknowledged** — `sd_notify`
  returns "datagram enqueued", and a store rejected for
  `FileDescriptorStoreMax=0` is *silently* dropped (`BARRIER=1`, v246, is the
  only synchronization primitive); one message passes at most 253 fds (Linux
  `SCM_RIGHTS` limit, `-E2BIG`); names not unique; `NotifyAccess=` gates who
  may store (default main process only).
- Forward-looking (recent systemd, informational): `FileDescriptorStorePreserve`
  propagation up through nested managers, soft-reboot pass-through, and
  kexec/Live-Update-Orchestrator preservation of memfds (v254/v262-era,
  `LUOSession=` v262). Not something to depend on; relevant only as evidence of
  where systemd is taking the mechanism.

## 4. What a reusable library/toolkit could add beyond systemd

Systemd's fdstore solves *manager-side retention for managed services*. The
gaps a toolkit can legitimately fill:

1. **Manager-less operation.** A user-session daemon started by hand (or by the
   first client) has no `NOTIFY_SOCKET`. Options: an *own-handoff* successor
   protocol (pass fds + state to the next instance via SCM_RIGHTS before
   exiting), exec-based self-succession (re-exec with inheritable fds), or a
   tiny holder process. Note systemd's own critique: lingering old instances
   under a *managed* service defeat lifecycle/resource/security accounting —
   own-handoff belongs outside managers, and fdstore belongs inside them. A
   toolkit can offer both behind one "retention" concept.
2. **Typed classification and claims.** `sd_is_*`-style checks (socket
   family/type, bound?, fifo, tty, regular, memfd) plus *verifiable manifest
   claims* per slot, so "bundle says slot `stdout` is a tty" is checked at
   adoption, not trusted. bus-proxy's adopt-time `local_addr()` check is the
   one-fd special case of this.
3. **Real addressing.** Stable slot names, generation/version counters, and
   bundle manifests — beyond non-unique FDNAME tags — with a defined collision
   policy against systemd's special names.
4. **Atomic bundle transfer.** Manifest + fds in one datagram (rkyv manifest is
   a natural fit and already in-tree via
   [clid-context](../../clid-context/src/lib.rs)), with byte-validation on
   receive (the existing `check-bytes` feature).
5. **Queue-preserving rendezvous.** Keep the submission socket either in a
   socket unit or fdstore'd, and the kernel receive queue becomes a durable
   inbox across restarts — datagrams carrying SCM_RIGHTS payloads sit in the
   queue (bounded by socket buffer; senders see `ENOBUFS`/`EAGAIN` when full).
   No userspace durable queue needed for the common case.
6. **State pairing.** A memfd + framed-state convention (write rkyv frames;
   `F_ADD_SEALS` when sealed) so restart continuity covers in-flight session
   bookkeeping, not just fds.
7. **Lifecycle policy helpers.** Store-per-session-fd on receipt (the seamless
   pattern systemd documents); `FDSTOREREMOVE` on *unrecognized* fds at init
   (survives code updates that stop recognizing an old slot — explicitly
   recommended by systemd); capacity management; `BARRIER` after critical
   stores when a manager is present.
8. **Safety.** Peer-credential checks (`SO_PEERCRED`/`SCM_CREDENTIALS`),
   per-peer fd/payload caps, provenance checks on received bundles.
9. **Test harness.** The fake-manager pattern (bind a `UnixDatagram`, assert
   payloads and fd round-trips) generalizes to fake-client/fake-successor
   harnesses for both handoff directions.

Explicitly *not* the toolkit's job: being a supervisor, replacing systemd
units, or inventing durable storage — it composes with a manager when one
exists and degrades honestly when none does.

## 5. Restarting a CLI stdio server mid-operation: feasibility and limits

Model recap: in clid the client exits right after one datagram; the daemon owns
the terminal-side fds. "Client operation continues" therefore means *the user's
terminal stays usable and in-flight output is not lost* while the daemon
restarts — not a live client connection (there is none).

What survives a daemon crash/restart **by kernel construction**:

- The **tty** is a shared device; nothing about the daemon dying disturbs it.
- **Pipe buffers** keep already-written bytes; the read side continues.
- **Children the daemon spawned** with dup'd fds (the actual command under
  execution) keep running — they hold their own fds and are independent of the
  daemon; only reaping/status bookkeeping is lost.
- **Queued submissions** in the rendezvous socket's receive queue, *if* that
  socket survives the restart (socket unit or fdstore'd, §4.5).

What dies with the daemon: userspace output buffers not yet written, handler
tasks mid-flight, session bookkeeping — recoverable only via serialized state
(§4.6) or by accepting the loss. Datagrams are unacknowledged: a crash between
receive and act loses that request with nobody to retry, because the client is
gone. Requiring one ack datagram before client exit is a real choice (still
compatible with an ultra-small blocking client) but changes client semantics;
defer to the user.

Three continuity mechanisms, honestly compared:

| Mechanism | Requires | Preserves | Notes |
| --- | --- | --- | --- |
| (a) systemd socket unit on the rendezvous socket (`ListenDatagram=`) | user/system unit | rendezvous socket + queued submissions, always | Restarts never refuse; service starts on first traffic. Per-session fds still die unless separately stored. |
| (b) fdstore'd self-bound rendezvous (bus-proxy pattern, generalized to a dgram socket) | `Type=notify` unit, `FileDescriptorStoreMax≥1` | same as (a), without a socket unit | Kernel-queue argument is the same; **unproven in bus-proxy's tests for dgram** (its tests cover the notify wire, and its production case is a TCP listener). A small integration probe on real systemd would close this. |
| (c) own successor handoff (SCM_RIGHTS to next instance, or re-exec) | nothing | rendezvous + per-session fds + serialized state, on *clean* restarts only | Crash-safe only for whatever was already handed off/stored. Outside a manager this is the native pattern; inside a manager it fights the manager (see §4.1). |

Feasibility verdict: **restart continuity for a stdio server is real and
cheap for the fd tier** — the tty/pipe/child survival is kernel-guaranteed,
and the rendezvous queue trick covers new submissions — while **in-flight
session state is a separate, optional tier** (state serialization) that should
be opt-in, not load-bearing. Stdio is genuinely just one slot kind here: the
same mechanics carry regular files the client opened, memfds, socketpairs,
and the client-side ends of pipes.

## 6. Reusable mechanisms and unresolved choices

Mechanisms worth extracting (behavior, not crate shape — decomposition and
naming are deliberately not decided here):

- **Ancillary send/receive** of N fds per datagram with `OwnedFd`-native
  ownership and `MSG_CMSG_CLOEXEC` on receive (generalize the current
  `sendfd`-crate single-purpose usage in
  [clid-client](../../clid-client/src/lib.rs)/[clid-daemon](../../clid-daemon/src/lib.rs)).
- **Activation capture**: pre-thread, PID-checked, env-consuming, probe-before-
  own — bus-proxy's `Manager::capture` is the reference shape; today's clid has
  no equivalent at all.
- **Adopt-or-bind-store-once decision**, generalized from one listener to a
  named-fd set (`LISTEN_FDNAMES` parsing + multi-fd adoption is the concrete
  widening of `requested()`'s exactly-one rule).
- **Notify wire functions**: `notify`, `fdstore(name, fd)`, `fdstore_remove`,
  `barrier`, watchdog heartbeat with scoped owner — bus-proxy has the first two
  plus watchdog; the rest are additive.
- **Fake-manager test harness** (from bus-proxy's tests) and, newly needed, a
  fake-successor harness for own-handoff.
- **Memfd state frames** + sealing, and **manifest encoding** (rkyv welcome;
  versioned).

Unresolved choices to settle with the user:

1. **Rendezvous**: filesystem path (discovery-friendly, current clid), abstract
   socket (no filesystem footprint, no stale-file problem), or both; and who
   owns it (socket unit, daemon-bound-then-stored, or transient).
2. **Transport**: datagram (current; message-aligned with SCM_RIGHTS, bounded
   payloads/queues) vs `SOCK_SEQPACKET` vs stream+framing (larger payloads,
   backpressure, but framing work and no natural 1:1 datagram↔fds shape).
3. **Ack semantics**: fire-and-forget client exit vs one ack datagram before
   exit (crash window vs slightly larger blocking client).
4. **Per-session fd retention**: store every received bundle fd (needs a large
   `FileDescriptorStoreMax` and an `FDSTOREREMOVE` discipline), successor
   handoff for active sessions, or accept the loss tier honestly.
5. **Deployment posture**: systemd user unit as primary (fdstore/socket-unit
   continuity) with manager-less degradation second, or the reverse. Affects
   which mechanism is the tested happy path.
6. **Blocking, sync client as the primary artifact** (matches the ask;
   bus-proxy demonstrates sync-before-runtime capture on the daemon side) with
   async as a wrapper, or both first-class. Current clid client is tokio-bound
   — a real delta, not a refactor detail.
7. **Varlink as future primary backend**: varlink is JSON-over-stream (the
   archived Rust crate is now sans-io) and has **no native fd passing** —
   SCM_RIGHTS stays a sidecar channel whatever the control protocol is. The
   useful decomposition is control plane (varlink: method calls, introspection
   via `org.varlink.service.*`, help — see below) vs fd plane (datagram +
   SCM_RIGHTS). Making the fd plane independent of the control protocol keeps
   that door open.
8. **Slot naming scheme** and collision policy with systemd special names
   (§2).

Future-work candidates captured (not designed, no tickets):

- **Shell completions** — inherent to the CLI surface once help/args settle.
- **Server-owned help/args with an ultra-thin forwarding client** — two
  distinct sub-candidates to keep separate: (i) *pure argv forwarding*: client
  forwards raw argv, help text lives server-side, client knows nothing (needs
  no protocol); (ii) *optional introspection protocol*: client can ask the
  server for a schema/help to render locally (this is where a varlink
  `org.varlink.service.GetInterfaceDescription`-style facility would live).
  Forwarding is the floor; introspection is an add-on.
- **One-off short-lived server startup when none exists** (low priority) —
  note it interacts with rendezvous ownership: a transient owner must hand the
  rendezvous to the long-lived instance, or the first-start race needs a lock.
- From the findings, small and later: `FDSTOREREMOVE`-on-unrecognized init
  pass; STATUS=-style progress reporting precedent (bus-proxy's open ticket).

## Verification

Implemented-vs-proposed for bus-proxy was established by reading its source,
tests, unit presets, maintained operations doc, and beads ticket states.
systemd semantics were read from the archived upstream checkout at `263~devel`
(most-recent master; version tags cited per claim: v219/233/236/246/254/262).
Not verified anywhere: fdstore replay of a **datagram** rendezvous socket
across a real restart (bus-proxy exercises the notify wire and a TCP listener
in production; §5b flags this as the one recommended probe before depending on
it). Varlink capabilities were read from the archived `varlink/rust` crate
README (sans-io architecture); no varlink implementation work exists in either
repo.
