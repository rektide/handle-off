---
type: Research
title: bus-proxy connection continuity — the implemented cooperative SSE handover line the public checkout does not carry
description: Recovery of implemented, live-demonstrated, and merely-proposed work preserving connected clients across opencode-bus-proxy restarts; distinguishes the public listener-only checkout from the compfuzor deployment continuation at /usr/local/src/opencode-bus-proxy-git where accepted-connection handover exists.
resource: design/fd-handoff/research1.glm53max.md
status: draft
generated: { by: "model:zai-coding-plan/glm-5.3#max", at: 2026-10-14 }
sources:
  - { resource: "opencode://ses_ec770da2cffelJ6om41fF87GS4", title: "This recovery session (happy-knight) — direction, investigation, and writing of this document" }
  - { resource: "opencode://ses_ed3b65203ffebXKv17uH4LiZrB", title: "silent-cactus — human direction for client persistence, handover naming, queue dial, live reset demonstration" }
  - { resource: "opencode://ses_ed36d5fdeffeQ2Ks5coMYJDORR", title: "Sol xhigh implementation of both handover modes, queue-zero, and isolated replacement evidence (cited by handover.md)" }
  - { resource: "opencode://ses_ed33ccef3ffefNULPTP6v370r9", title: "Astra completion-checkpoint protocol design (cited by completion-checkpoints.astra.md)" }
  - { resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/doc/event-multiplexer/handover.md", title: "Maintained cooperative SSE handover instructions, modes, contract, and recorded live demonstration" }
  - { resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/design/event-multiplexer/active-client-handover.md", title: "Handover registry proposal — material inventory, memfd manifest, update cadence, crash-consistency limits" }
  - { resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/design/event-multiplexer/completion-checkpoints.astra.md", title: "Proposal — publication barriers, admission leases, client acknowledgments" }
  - { resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/design/event-multiplexer/state-exports.sol-medium.md", title: "Typed registry export prototypes — JSON, tracing, Arrow/Polars, optional rkyv" }
  - { resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/cli/daemon/src/handover.rs", title: "Implemented systemd transaction — per-descriptor stores, busctl store verification, exit 75" }
  - { resource: "file:///usr/local/src/opencode-bus-proxy-git/repo/components/gateway/src/handover.rs", title: "Implemented freeze/import coordinator, image format, buffered/boundary modes" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/crates/service-manager/src/lib.rs", title: "Public checkout's listener-only bind/adopt/store-once policy" }
  - { resource: "file:///home/rektide/src/opencode-bus-proxy/.beads/issues.jsonl", title: "ocbp-* ticket record — no connection-preservation ticket exists in either beads store" }
  - { resource: "file:///home/rektide/.config/systemd/user/opencode-bus-proxy.service.d/handover.conf", title: "Installed live drop-in — FileDescriptorStoreMax=2048, buffered mode" }
  - { resource: "design/fd-handoff/research0.glm53max.md", title: "Prior fdstore/handoff research whose bus-proxy scope this document corrects" }
---

# bus-proxy connection continuity — recovery

Situation: the fresh handle-off reconception of this repo wants to preserve
many kinds of live handles/FDs across server restarts. The commissioning
operator remembers bus-proxy work "for more than listener" — preserving
connected clients across restarts. A prior investigation
([research0](research0.glm53max.md)) examined `/home/rektide/src/opencode-bus-proxy`
and correctly reported that checkout stores exactly one listener with no
memfd/rkyv state. This session was asked to independently recover intent,
history, and any abandoned or unpushed work. Ask: what exists, where, at which
tier, implemented or planned.

**Headline finding: the memory is real.** Established-connection preservation
was designed, implemented, isolated-tested, and live-demonstrated on
2026-10-11 — but in a **different checkout**: the compfuzor-managed deployment
continuation at `/usr/local/src/opencode-bus-proxy-git/repo`, whose `origin`
remote is the *local* public checkout (a file:// remote), with its `main`
bookmark still at the public tip `3d9410c5`. None of it was ever pushed to
`github.com/rektide/opencode-bus-proxy`, filed in beads, or carried into the
dev checkout at `~/src/opencode-bus-proxy`. research0's "not present anywhere"
claims are accurate for the public checkout only.

## 1. Where the work lives — two checkouts, one lineage

| | Public/dev checkout | Deployment continuation |
| --- | --- | --- |
| Path | `/home/rektide/src/opencode-bus-proxy` | `/usr/local/src/opencode-bus-proxy-git/repo` (wrapper: `bin`, `env`, `etc`, compfuzor `links.json`) |
| jj `main` | `3d9410c5` (2026-10-11 07:08, pushed) | same `3d9410c5`, plus ~30 unpushed local commits through 2026-10-11 23:56 |
| `origin` | `git@github.com:rektide/opencode-bus-proxy` (+ `voodoo` archive) | `file:///home/rektide/src/opencode-bus-proxy` |
| Restart story | listener retention only | listener retention **plus cooperative established-SSE handover** |
| Layout | `crates/{protocol,router,gateway,service-manager,discovery,daemon}` | reorganized: `components/{protocol,router,gateway}`, `cli/daemon`, `telemetry/{core,arrow,clickhouse-native}` |
| Beads | 20 `ocbp-*` tickets, no connection-preservation ticket | same 20 tickets carried over; **no ticket was ever filed for handover** — tracking lives in sessions and commits |

The triggering session is `ses_ed3b65203ffebXKv17uH4LiZrB` (petname
silent-cactus, titled "Bus-proxy recovery and connection persistence across
service restarts", 2026-10-11, recovered from the live OpenCode session store
at `~/.local/share/opencode/opencode-local.db`). The operator's opening ask
there is nearly verbatim the remembered intent: *"persist all our clients and
pass them off to a new copy of ourselves, upgrade ourselves without breaking
interruption. systemd fdstore use to persist… suspend all connections in
fdstore."*

## 2. Tier-by-tier account (the distinctions the commission asked for)

### Tier 0 — Listener retention (implemented, public, shipped)

The public checkout's entire story, implemented 2026-10-11 in commits
`ac9fd399` (socket-activation adoption), `7f901288` (fdstore
`FDSTORE=1\nFDNAME=listener`, store-once invariant), `3c3d3cbf` (hardened
preset, `FileDescriptorStoreMax=1`); tickets `ocbp-systemd-activation`,
`ocbp-systemd-fdstore`, `ocbp-systemd-units-hardened` (all closed). Maintained
account: [operations.md §Descriptor
store](file:///home/rektide/src/opencode-bus-proxy/doc/event-multiplexer/operations.md).
Queued *connects* survive the restart (kernel backlog); established streams
are live-only and end with their process. This is everything research0 saw.

### Tier 1 — Bounce/reconnect (implemented, deployment line only)

`bbc5fbfa` *"fix: abort retired SSE feeds so origin outages trigger client
retries"* (2026-10-11 18:52). Origin loss now aborts client SSE connections
(transport error, not clean end-of-stream) so clients retry; the proxy stays
alive and reconnects upstream itself. The public checkout predates this and
still closes streams cleanly (its README: "Source failure closes downstream
streams"). This is *client-driven recovery* — connections still break.

### Tier 2 — Cooperative established-connection handover (implemented, isolated-tested, live-demonstrated)

The centerpiece. Maintained instructions:
[`doc/event-multiplexer/handover.md`](file:///usr/local/src/opencode-bus-proxy-git/repo/doc/event-multiplexer/handover.md)
(whose frontmatter cites the Sol implementation session
`ses_ed36d5fdeffeQ2Ks5coMYJDORR`). A **planned, cooperative cut** — not crash
recovery — triggered by **SIGUSR2** (requires `--handover-unit`):

- **What transfers** (exact, not reconstructed): the listener; the
  **upstream** plain-HTTP source TCP socket *and* its HTTP chunk/length/close
  framing, unread userspace bytes, partial SSE decoder state, source epoch,
  and same-boot inactivity deadline; **eligible downstream SSE TCP sockets**
  with, in buffered mode, dequeued output and its precise unsent byte offset —
  including mid-chunk-header/mid-payload/mid-trailer cuts. Subscription IDs,
  pending/active state, effective Session interests, queue order, opening
  frames, counters, and the runtime queue target all carry over. Descriptors
  pass under generation-scoped `FDNAME`s (e.g. `bp-g7-down-c42`,
  `bp-g7-manifest`), resolved by name through `LISTEN_FDNAMES`, never old fd
  numbers.
- **Two modes**: `buffered` (default; exact bytes incl. stalled clients) and
  `boundary` (alias `unbuffered`; stop admission, drain all admitted backlog
  to a clean chunk boundary within `--handover-drain-ms` 0–30000, close
  stragglers with an incomplete-response error). No continuous per-client
  payload copy in either mode — writers retain the router's shared immutable
  `Bytes` plus a chunk header, static trailer, and write offset; vectored
  writes join them; flattening happens only while parked (buffered).
- **What does not transfer — deliberately**: ordinary HTTP exchanges,
  WebSocket tunnels, HTTP/1.0, pipelined read-ahead, and SSE responses on
  Hyper-owned/reused connections are closed at the cut. HTTPS upstream is
  rejected during preparation. Only a first authorized HTTP/1.1 GET of
  `/api/event` / `/api/experimental/event` is a transferable downstream.
- **Mechanics** (daemon side,
  [`cli/daemon/src/handover.rs`](file:///usr/local/src/opencode-bus-proxy-git/repo/cli/daemon/src/handover.rs)):
  image in a **sealed memfd** (`libc::memfd_create`, seals against
  write/grow/shrink; version-1 JSON/base64 envelope with SHA-256 integrity,
  same-boot identity, compatibility digest, 256 MiB cap); descriptors offered
  individually with unique names and `FDPOLL=0`, manifest last; `BARRIER=1`
  orders notifications, and **acceptance is separately verified by name and
  device/inode identity** through `org.freedesktop.systemd1.Service.DumpFileDescriptorStore`
  over busctl (verified against systemd 262); commit by **exit 75** under
  `Restart=on-failure`/`always`; successor probed first with
  `--check-handover-format 1`; the successor **invalidates transient store
  entries and verifies their removal before any resumed I/O**
  (`FDSTOREREMOVE=1`, [`systemd.rs`](file:///usr/local/src/opencode-bus-proxy-git/repo/cli/daemon/src/systemd.rs#L189)).
- **Live queue dial**: authenticated `PUT /_bus/queue` `{"frames":N}` accepts
  **zero** (no waiting backlog; one cancel-safe ready-receiver credit; one
  bounded handshake slot for opening/activation frames); lowering drains
  rather than discards; the selected target survives handover but not crash.
- **Bootstrap honesty**: an old incapable process cannot export clients — one
  ordinary bounce is required after installing the new binary; only subsequent
  resets preserve.
- **Evidence**: isolated `--ignored` systemd suite
  (`cli/daemon/tests/handover_systemd.rs`) replaced the process twice per mode
  preserving the same consuming response, controlled ID/interests, runtime
  zero target, and upstream socket; gateway/router tests cover every chunk
  framing cut, decoder fragments, descriptor reordering/missing/corruption,
  lease abandonment, queued TCP connects, boundary timeouts. **Live
  demonstration, 2026-10-11**: installed user service
  `~/.config/systemd/user/opencode-bus-proxy.service.d/handover.conf`
  (`FileDescriptorStoreMax=2048`, buffered mode); SIGUSR2 reset replaced PID
  3776832 → 3797409 with a legacy and an activated controlled SSE response
  staying open across the replacement, identical downstream and origin socket
  inodes before/after, a post-reset interest PUT on the original subscription
  ID succeeding, `kept=2 dropped=0`, store returning to the single listener.
  Probe retained at `/home/rektide/tmp-opencode/silent-cactus-live-handover.py`.
  The service is **still deployed and running today** (checked 2026-10-14:
  active, MainPID 1805, `NFileDescriptorStore=1`, handover flags present in
  `/usr/local/bin/opencode-bus-proxy --help`).

Commit series (jj, deployment repo): design inventory `0dd94bbd`, naming
`5de9c12b`, registry prototypes `d14ca9e6`, router freeze + decoder import
`cc8d9615`, resumable HTTP/1.1 SSE transports `4c9a6e86`, systemd
transaction `c7551353`, cancellation-safe custody `9b0e431b`, image
amplification bound `8433f72a`, chunk-cut tests `13d11fd7`, vectored shared
payloads `a1782536`, live queue targets `ec9eeae7`, quiesce + boundary
`343f6b36`, capability honesty `a7900d08`, mode/ops docs `4e57d10e`, live
receipt `1ed1c5c6`, integration merge `287d502e`. Implementation today:
`components/gateway/src/handover.rs` (729 lines),
`components/gateway/src/transport.rs` (849 — resumable SSE transports),
`cli/daemon/src/handover.rs` (346), tests 760 + 342 lines.

### Tier 3 — Application/protocol completion checkpoints (designed only, never built)

[`completion-checkpoints.astra.md`](file:///usr/local/src/opencode-bus-proxy-git/repo/design/event-multiplexer/completion-checkpoints.astra.md)
(commit `4bb205ed`, status *proposal — research only*): origin→proxy
**publication barriers**, **admission leases** (hold new work for quiesce),
optional **client apply acknowledgments**; grounded in inspected bus-smart
seams; key honesty rule "execution idle ≠ responses delivered ≠ client state
updated". The session's final recommendation — the first slice to build — was
an operational **`prepare-handover` checkpoint**: origin emits an ordered
barrier on OBP's negotiated stream → proxy parks source intake at that
boundary → preparation returns a token tied to epoch+parked state → commit
performs the existing fdstore handover, abort resumes. Not implemented.

### Crash recovery — explicitly out of scope, by argument

[active-client-handover.md](file:///usr/local/src/opencode-bus-proxy-git/repo/design/event-multiplexer/active-client-handover.md#L115)
documents why connection handover is cooperative-only: periodic snapshots go
stale against socket reads/writes (replaying an old offset duplicates bytes;
omitting consumed bytes loses them); even per-operation memfd updates leave a
crash window between syscall and checkpoint. "That requires a surviving I/O
owner or another explicit crash-consistency mechanism." A crash inherits the
listener plus startup defaults and a fresh source epoch; the successor
actively refuses stale images. "No exactly-once, crash-continuous, or
power-loss recovery claim."

### Registry observability (prototyped; adjacent, not load-bearing)

`state-exports.sol-medium.md` + commit `d14ca9e6`: one typed registry feeding
JSON inspection, reactive tracing attributes, Arrow IPC via Polars lazy
(Polars 2.0.0 ↔ PyArrow 26.0.0 interop verified), and an **optional** rkyv
state image. handover.md is explicit that Arrow/Polars/rkyv are *not* required
or enabled by the handover mechanism. The later `telemetry/` work
(OTLP, ClickHouse-native) continues this export line, not connection
continuity.

## 3. Corrections to research0

[research0 §1](research0.glm53max.md) states "Not present anywhere: memfd
state serialization, `FDSTOREREMOVE=1`, `BARRIER=1`, multi-descriptor stores,
`LISTEN_FDNAMES` parsing… the store is exactly one named listener." That is
true of `/home/rektide/src/opencode-bus-proxy` and false of the workspace as
a whole: the deployment continuation implements *every one* of those
codepoints (sealed memfd image; `FDSTOREREMOVE=1`; `BARRIER=1`; per-descriptor
named multi-stores with `FileDescriptorStoreMax=2048`; `LISTEN_FDNAMES`
multi-fd adoption with duplicate/incomplete rejection). research0's §4/§6
"toolkit could add" list (typed claims, slot addressing, atomic manifest+fds,
memfd state frames, store-acceptance verification, capacity management) is
therefore not hypothetical — it has a working in-workspace precedent at
production quality, with the exact reservations research0 raises (crash
windows, bootstrap, capacity accounting: "clients + source + manifest +
retained listener").

## 4. What the operator is most likely remembering

Ranked by evidence:

1. **The silent-cactus session and its live reset** (`ses_ed3b65203…`): two
   SSE clients surviving a PID replacement with identical socket inodes and a
   surviving subscription ID — the exact "more than listener" experience.
2. **The vocabulary invented there**: *handover*, *handover manifest*,
   *state image*, memfd descriptor registry, "quiesce mode", the queue dial
   to zero, `prepare-handover` checkpoints — a rich design conversation the
   same evening.
3. **Possibly the public ticket's phrasing**, `ocbp-systemd-fdstore`
   ("seamless self-upgrade with no connection-refused window") and
   operations.md's "queued connections survive the restart" — listener/backlog
   tier wording that reads stronger than it is.
4. The follow-up **simplification session** (`ses_ed07ec45…`, 2026-10-12:
   "reduce the mental complexity of these crazy things… break up more into
   crates") — which is the proximate origin of this repo's decomposed-toolkit
   reconception; the handover line is its direct ancestor.

## 5. Relevance to handle-off (kept short by commission)

The OBP handover is the workspace's largest prior art for the handle-off
program and a ready source of validated mechanisms and honest limits:
generation-scoped fd naming; sealed-memfd manifest with integrity/compat
digests and same-boot identity; exact-offset transport transfer;
buffered-vs-boundary cut modes; store acceptance verified by kernel identity
(not send success); invalidation-before-I/O on the successor; the bootstrap
problem for pre-existing processes; commit-via-exit-status under a restart
policy; and the crash-consistency argument for why cooperative cuts and crash
recovery are different contracts. Its unresolved remainders map onto open
handle-off questions: application-state quiesce (Tier 3), HTTPS/TLS state
(non-transferable), tunnels and non-SSE protocols (closed at cut), and
cross-version successor negotiation.

## Verification

Both checkouts' source, tests, unit presets, and maintained docs were read;
jj/git histories and bookmarks compared (`jj log`, `jj bookmark list`,
`jj git remote list`, `jj op log` for abandons); beads stores in both repos
inspected (identical 20 tickets, none for handover); the silent-cactus session
recovered from the live OpenCode session store and cross-checked against its
cited commits and docs; the deployed user service, drop-in, running state
(`systemctl --user show`), and installed binary's `--help` verified
2026-10-14; bus-smart's canonical architecture and the founding
rust-proxy0.glm53.md design (which explicitly chose "a proxy restart looks
like a connection drop") checked for contrast. Not done here: running the
isolated systemd handover suite, live-signaling the deployed service, or any
upstreaming — all out of a read-only recovery commission's scope.
