---
type: Assessment
title: Named resources, visible handovers, and state that outlives processes
description: Current naming audit and a broad opportunity assessment connecting the shared systemd transcript to handle-off's implemented contracts and possible future applications.
resource: /design/fd-handoff/naming-opportunities0.gpt6a.md
status: draft
generated: { by: "model:openai/gpt-6-astra#high", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec616a6eaffe0I3U3HgN4tJ0Ho"
    title: "Human naming and opportunity questions; transcript recovery, implementation audit, upstream checks, and assessment"
  - resource: "https://chatgpt.com/share/6acf6d53-88e8-83e9-b7a8-8112df46c43c"
    title: "Commissioned source conversation: fdstore and LUO grouping, retained state, and richer service reporting"
  - resource: /handles/core/src/lib.rs
    title: "Caller-scoped ResourceId and handle tables"
  - resource: /handles/bundle/src/lib.rs
    title: "Generation, bundle, member, and exact manifest membership contracts"
  - resource: /state/core/src/lib.rs
    title: "Named versioned extensions and payload carriers"
  - resource: /systemd/generation/src/name.rs
    title: "Opaque namespace/generation tokens and exact manager-address grammar"
  - resource: /systemd/notify/src/lib.rs
    title: "Existing generic notification transport and supervision boundaries"
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: "Capability decomposition, application ownership, and planned epics"
  - resource: /design/fd-handoff/lifecycle0.gpt6a.md
    title: "Transient generations, persistent listeners, aliases, and authority decisions"
  - resource: /doc/systemd/generations0.gpt61s.md
    title: "Implemented catalog and manager deduplication limits"
  - resource: /doc/handover/direct0.gpt61s.md
    title: "Implemented offer-scoped control and single-bundle process evidence"
  - resource: /README.md
    title: "Current workspace capability and sealed-carrier inventory"
  - resource: /.beads/issues.jsonl
    title: "Existing handoff inspection, compatibility, Varlink, and manager work anchors"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/man/systemd.service.xml"
    title: "Read locally: per-service custody, manager forwarding, and multiple LUOSession names"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/man/sd_notify.xml"
    title: "Read locally: lifecycle messages, pidfd identity, deadlines, and restart reset"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/src/shared/varlink-io.systemd.Metrics.c"
    title: "Read locally: counter, gauge, string, object, Describe, and List schemas"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/docs/FILE_DESCRIPTOR_STORE.md"
    title: "Read locally: resource pass-through, kernel replacement, manager forwarding, and custody lifetime"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/man/systemd-report-files@.service.xml"
    title: "Read locally: text-file reporting adapter and discovery"
  - resource: "https://docs.kernel.org/userspace-api/liveupdate.html"
    title: "Retrieved 2026-10-14, rendered 7.3.0-rc7: named LUO sessions, resource tokens, restoration, compatibility"
  - resource: "https://docs.kernel.org/core-api/liveupdate.html"
    title: "Retrieved 2026-10-14: LUO grouping and resource-specific preservation lifecycle"
  - resource: "https://docs.kernel.org/mm/memfd_preservation.html"
    title: "Retrieved 2026-10-14: preserved seals, sparse expansion, CLOEXEC reset, and unstable compatibility"
---

# Named resources and possible futures

**We already name the important things. The opportunity is to make those names
useful across inspection, application recovery, and service operation.** The
strongest near-term direction is an explainable resource inventory joined to
cooperative handover; the more ambitious direction is retained application state
whose useful lifetime is longer than the process running today's code.

This is an assessment of possibilities, not selection of new implementation
scope. The human clarified that multiple bundles were a general aspiration, not
a previously worked-out scenario. The completed direct adapter need not be
redesigned merely to satisfy an expansive interpretation of that aspiration.

## 1. What we name today

| Thing | Actual representation and scope | What is still application policy |
| --- | --- | --- |
| Resource | `ResourceId(String)`, unique within an inventory | Meaning, assignment, cross-generation stability, relationship to an actual open resource |
| Bundle | `BundleId(String)`, unique within a manifest | Whether a bundle is a session, dataset, job, connection, or another grouping |
| Member | `MemberId(String)`, unique within its bundle, referring to a resource | Role vocabulary; `stdin`, `stdout`, `stderr` are existing conventions |
| Prepared handover | `GenerationId(String)` | Freshness, ownership domain, and lifetime; it is not automatically an ordered counter |
| State extension | String key at member, bundle, or manifest scope | Which formats are required and how they restore application behavior |
| State format | `StateFormat { name, version }` | Schema authority, compatibility, converters |
| systemd address | Namespace token + generation token + resource slot/root/marker | Token generation, namespace ownership, binding to the logical manifest |
| Rendezvous endpoint | Configured filesystem or abstract Unix address | Discovery policy, connection, authentication, liveness |

The string IDs currently require nonempty strings without control characters.
They are exact strings, with no case folding, hierarchy, Unicode normalization,
URI semantics, or global registry. A slash can be part of an ID; it does not
create containment behavior. The core does not impose a universal length bound;
individual wire/adoption layers apply their own limits.

For example, the following are useful **application conventions**, already
expressible without changing the foundation:

```text
generation: upgrade-42
  bundle: cli/request-17
    member: stdin       -> resource: pipe/input-17
    member: stdout      -> resource: tty/console-17
    member: stderr      -> resource: tty/console-17
  bundle: busprox/downstream-83
    member: connection  -> resource: socket/downstream-83
    extension: writer   -> format: busprox.pending-write, version: 1
  bundle: cache/catalog
    member: image       -> resource: memfd/catalog-12
```

Repeated member references can deliberately share one resource. Reusing a string
across generations does not prove that two FDs reference the same open file
description; that continuity needs separate evidence.

The physical manager tags remain deliberately compact:

```text
ho1.<namespace-32hex>.<generation-32hex>.m          root
ho1.<namespace-32hex>.<generation-32hex>.c          commit marker
ho1.<namespace-32hex>.<generation-32hex>.r00000000  resource slot 0
```

They are transport addresses, not attractive application names. The manifest
maps resource slots back to logical IDs. `ho1` is the existing wire family;
the tracker rename to `handoff-*` did not rename it or the `handle-off-*` crates.

### Recommended naming direction

Keep readable scoped identities above opaque backend addresses. Let applications
use their native IDs, and document a small convention for roles and state-format
ownership. Introduce a separate optional label only if humans need a mutable
display name alongside a stable ID. Qualify identities with their application or
service scope when aggregating inventories; do not pretend `connection-1` is
globally unique.

Use tuples/structured fields for cross-service references before inventing a
universal URI grammar. If names become persistent lookup keys, explicitly decide
assignment, reuse after deletion, equality, and rename behavior. Metrics and
Varlink interface names need their own provider-qualified namespace; they need
not force that grammar onto every bundle/member ID.

## 2. The transcript's useful expansion

The shared conversation has three substantive answers: ordinary stores versus
LUO sessions; lifecycle and structured reporting; resource survival across
process, userspace, and kernel replacement. Its embedded citation handles are
not portable evidence. This assessment independently checked selected claims in
the local systemd archive at `54b24a3855de37e5e19235c44acd113eb6ce6830`
(`263~devel`) and current kernel documentation rendered as `7.3.0-rc7`.
These are source/documentation observations, not conformance tests on this host
or verification of every historical version claim in the transcript.

Four distinctions organize the opportunities:

1. **Retention domain:** systemd has one fdstore per service unit. Logical groups
   do not get independent manager quotas, retention policies, or access control.
2. **Application meaning:** manifests can describe many bundles and state images.
   Their recovery and execution rules remain application-owned.
3. **Transition:** process restart and same-kernel soft reboot retain references;
   LUO reconstructs supported objects across a prepared kernel replacement.
4. **Visibility:** supervisory messages, current facts, live observation, and
   historical events are complementary views, not interchangeable evidence.

Upstream supports multiple `LUOSession=` names, but these are actual kernel
preservation sessions, not extra ordinary fdstores. Applications should not need
LUO just to distinguish several caches or connections. Ordinary named memfds are
already separate resources within the one store.

## 3. Opportunity: a resource inventory you can actually understand

**Payoff:** answer “what is being kept alive, for whom, and can the next version
use it?” rather than just dumping raw FD numbers.

A view could join:

- service/application scope and observation time;
- logical bundle/member/resource IDs and state-format versions;
- declared aliases versus independently observed handle properties;
- generation, custody location, and current handover phase;
- required state, missing entries, unsupported formats, and adoption reasons;
- state-image bytes, descriptor counts, and manager capacity observations.

An illustrative rendering, not an implemented command:

```text
busprox.service / generation upgrade-42
  downstream-83
    connection   socket/downstream-83     staged, dormant
    writer       busprox.pending-write/1  restorable
  catalog
    image        memfd/catalog-12        sealed, 8 MiB
  execution      old owner parked; successor not released
```

Important source boundary: systemd inspection can expose names/metadata, but
does not hand arbitrary clients the retained FDs. A rich external view needs an
application-supplied catalog, a sanitized metadata export, or the application's
own inherited inventory. It cannot magically decode a root memfd from `busctl`.
Attach evidence origin and observation generation so a stale manager snapshot is
not presented as current application authority.

This advances existing `handoff-inspection-inventory`,
`handoff-inspection-adoption`, and `handoff-inspection-exports`. It is useful
before implementing kernel handover or a universal management service.

## 4. Opportunity: one model, several useful reporting surfaces

**Payoff:** see handovers from a shell, collect them with system tools, and watch
them progress without inventing unrelated status schemas for each interface.

Use an optional read-only view model, projected into:

| Surface | Useful content |
| --- | --- |
| `STATUS=` | One sentence: “Prepared 12 bundles; waiting for successor compatibility” |
| `io.systemd.Metrics` | Counts, state bytes, quarantine count, last outcome, coherent generation objects |
| Custom Varlink | Inventory, compatibility explanations, snapshot-then-watch updates |
| Structured journal | Prepare, rejection, relinquishment, release, retirement, recovery decisions |
| Plain text/JSON | Local inspection, support bundles, and easy scripting |

The checked Metrics schema supports `counter`, `gauge`, `string`, and arbitrary
JSON `object` values. Its `List` and `Describe` are multi-reply enumerations, not
permanent watches. A custom watch needs its own revision/cursor, ordered initial
snapshot, gap handling, and reconnect semantics. These semantics would also help
busprox and thin-client observers remain understandable through replacement.

An initial reporting experiment could atomically replace a small public summary
file for `systemd-report-files`; that exports UTF-8 text, not a typed JSON object
even when the text happens to contain JSON. A native Metrics provider is the
richer follow-on. Reporting itself remains an experimental systemd component.

Public summaries should be designed explicitly: names and metadata can be
visible even when the underlying payload is private. In particular, do not use
invocation environments or client request contents as convenient metric labels.
Aggregate metrics by default; put high-cardinality per-connection inspection in
the detailed inventory API.

**Architectural choice:** Varlink reporting can be valuable independently of
Varlink FD transfer. The current `handoff-varlink-*` tickets primarily investigate
delivery. Observation need not wait for a chosen descriptor-transfer extension,
nor become a hidden requirement of the tiny local client.

## 5. Opportunity: explainable upgrade compatibility

**Payoff:** ask the candidate successor whether it can reconstruct this workload
before making the irreversible ownership change.

The current extension key, format/version, and `required` flag are a foundation
for an adoption report. A proposed read-only assessment could say:

```text
cli/request-17       supported
busprox/downstream-83 supported: writer format v1
cache/catalog        blocked: requires catalog format v3; successor supports v2
```

Separate declared compatibility from dormant reconstruction checks and from real
readiness. A preflight against a changing live inventory needs a snapshot ID;
the final parked capture must be revalidated if the relevant state changed.
Converters can later be application-provided, producing a fresh state image
while old data remains available until the chosen commit boundary.

This gives `handoff-coordination-compatibility` a concrete purpose. It also gives
multi-bundle testing a meaningful scenario: one unsupported bundle blocks a
generation-wide release; abort retires all reconstructed copies. That is more
informative than merely changing a fixture's resource count from one to two.

## 6. Opportunity: persistent named state alongside transient handovers

**Payoff:** warm caches, indexes, routing tables, or compiled artifacts survive
code replacement without being rebuilt solely because a process exited.

There are two genuinely different designs:

- **Immutable retained images:** versioned sealed memfds plus an explicitly
  selected committed inventory. This composes with today's carrier and positional
  state-range references. It is a good first demonstration for an index/cache.
- **Mutable shared backing:** a `MAP_SHARED` arena or append-only log maintained
  during operation. This needs interruption-safe updates, committed roots or a
  journal, remapping, and reconstruction of process-local state. Today's memfd
  carrier is sealed against writes and is not such an arena.

The important new concept is a resource lifetime spanning several handover
generations. Current lifecycle design deliberately keeps persistent listeners
outside transient v1 generations. Extending that idea to stable named datasets
needs an explicit retained-resource policy, not merely a longer-lived generation.

There is a concrete manager constraint: storing the same open resource under a
new generation tag may be deduplicated before the new name is assigned. Thus
“republish every unchanged image under every generation's name” is not a valid
baseline. Options include stable custody entries referenced by a new manifest
design, or fresh image resources per generation. Reusing images reduces copying
but needs explicit ownership/reference accounting and deliberate retirement;
fresh images are simpler but consume more bytes and FD slots.

No automatic garbage collector is implied. Immutable image IDs also do not prove
content identity; a digest can be a useful optional verification/cache key for
immutable bytes, whereas a live socket cannot be content-addressed this way.

## 7. Opportunity: a restart-survival assessment

**Payoff:** report what a particular operation can preserve before attempting it,
and distinguish warm recovery, partial recovery, and cold start afterward.

| Operation | Useful question |
| --- | --- |
| Cooperative direct successor | Are all required handles and state reconstructible under this offer's release? |
| Service replacement | Is manager custody complete, with the intended retention policy and incarnation evidence? |
| Inner-manager replacement | Is the enclosing manager configured to forward and retain this inventory? |
| Soft reboot | Will the same-kernel resources and service identities survive the userspace transition? |
| LUO kernel replacement | Are these exact resources and source/target versions supported, and is the state representation suitable? |

Represent observations as supported, unsupported, or unknown with grounds, rather
than inventing a single `restart_safe` boolean. Required versus rebuildable data
and cross-bundle dependencies must be application declarations.

A particularly useful finding: a mixed generation might retain its root/marker
memfds across LUO while its sockets disappear. Our exact membership checks should
reject that generation. Marker presence alone must not allow partial execution.
If an application wants to recover the cache but reconnect clients, it needs an
explicit alternative recovery plan. Silently weakening generation validation
would confuse continuity with a cold or partially warm restart.

This could start as inspection and example policy rather than another core trait.
Record descriptor budget and image logical bytes separately: fdstore capacity is
a descriptor count, not a RAM budget.

## 8. Opportunity: kernel-update-ready state images

**Payoff:** an expensive in-memory dataset comes back warm even after a prepared
kernel update, with ordinary fdstore still serving as the application interface.

The checked systemd sources document automatic LUO preservation of supported
memfds. The kernel documentation confirms named sessions and resource tokens,
while explicitly declining universal cross-version compatibility. The current
memfd documentation preserves known seals, contents, size, and file position;
holes allocate pages, hugetlb memfds are unsupported, and raw restoration does
not preserve CLOEXEC. Our sealed-carrier adoption should continue checking its
requirements rather than assuming an older/newer kernel pair preserves them.

An attractive experiment is a bounded immutable search index or compiled cache:
load it, retain it, replace the service, then in a suitable isolated system test
soft reboot and an exact LUO kernel pair. Measure actual rebuild avoided, logical
image size, restoration checks, and remaining live-resource losses.

Compact chunks or a compact handover image are more promising than a giant sparse
arena. Chunking also increases descriptor pressure, so packing several state
ranges into an image is a meaningful design option already supported by our
payload model. Current individual image limits are a practical constraint, not
proof that the carrier scales to arbitrary database-sized heaps.

Explicit `LUOSession=` integration becomes interesting when an application needs
its own preservation groups/token map and restoration/finish lifecycle. It should
be a separate optional capability. Multiple sessions do not create independent
ordinary service retention policies, and they do not expand kernel support to
arbitrary connection/device types. Bootstrap discovery of the token map and
cleanup after partial restoration would need a concrete design.

## 9. Opportunity: supervision that understands actual progress

**Payoff:** a lengthy capture/restore can explain its progress and use lifecycle
deadlines correctly, without declaring success too early or masking a stuck loop.

The existing notifier already permits raw assignments. Useful optional typed
helpers and application examples include notify-reload's monotonic timestamp,
timeout extension, pidfd-based main-process updates, watchdog eligibility tied to
real progress, and restart-backoff reset after meaningful successful operation.
These are protocol conveniences and integration policy, not missing descriptor
transport mechanics.

Keep the service boundary precise: replacing an internal worker is not necessarily
stopping the service. Upstream documents `STOPPING=1` as irreversible since v259.
`READY=1` does not grant a handover execution lease, watchdog progress does not
prove custody, and a pidfd identity update does not prove the old writer stopped.
Map lifecycle transitions deliberately instead of wiring similarly named events
together automatically.

## 10. More adventurous compositions

### Selective worker or shard replacement

Named bundles could let a service replace one shard while the rest keeps serving.
This is attractive for busprox or worker pools, but requires defining independent
authority domains. Shared upstreams, state carriers, writers, or admission paths
may make apparently separate bundles inseparable. Start with a dependency graph
and identify the groups that can actually quiesce independently. Current direct
control coordinates one offer; it does not provide per-bundle release, partial
commit, or a multi-offer transaction.

### A resource keeper and replaceable compute workers

A stable service could own listeners/shared state while replaceable workers
receive selected handles. This helps environments without usable manager custody
and workloads needing resource redistribution during normal operation. It also
creates another long-lived owner, protocol, access boundary, and recovery problem.
systemd's ordinary fdstore is not an arbitrary fetch-by-name broker. Assess this
as an application architecture, not a prerequisite for the toolkit.

### Upgrade rehearsal and failure explanations

Launch a candidate, give it the captured manifest/inventory, reconstruct dormant,
produce a compatibility report, then abort and verify retirement. This exercises
the real adopter rather than a separate validator. Dormant reconstruction must
avoid shared socket/file side effects, so this is not unrestricted shadow
execution against live handles. Offline checks can use immutable images more
freely. An event timeline can make failed rehearsals actionable: which bundle,
which format, which missing custody observation, and which owner remained parked.

### Repeated replacement as the demonstration

The most compelling user experience may be a CLI command that keeps printing, or
a busprox stream that keeps flowing, while the server is replaced repeatedly.
Combine that with the named inventory and timeline so the continuity is visible
and explainable. A separate cache demonstration covers state surviving beyond
processes. These show more useful value than a universal API designed before its
consumers exist.

## 11. Suggested ordering and decisions

| Direction | Existing footing | What unlocks the payoff |
| --- | --- | --- |
| Naming conventions + readable inventory | Strong: core IDs, catalog, inspection tickets | An application-supplied view joining logical names and observed custody |
| Compatibility/adoption explanations | Strong: required versioned extensions | Concrete format support and dormant-restore assessment |
| Varlink observation + Metrics export | Upstream interfaces checked; local integration unbuilt | One optional view model, provider naming, and observation semantics |
| Real manager replacement | Catalog/primitives/core built | Publication/retirement, incarnation evidence, real-manager conformance |
| Persistent immutable datasets | Sealed carrier and manifests built | Stable custody lifetime and generation reuse/retirement policy |
| Soft reboot / manager-chain survival | Upstream mechanism documented | Isolated deployment and lifecycle conformance evidence |
| LUO-backed warm state | Upstream mechanism documented | Exact supported kernel pair and explicit image/recovery contract |
| Mutable arenas / selective migration / resource keeper | Architectural possibilities | A consumer whose needs justify the additional consistency and authority model |

My recommended first design conversation is **which named inventory should the
application expose, and which lifetimes do its resources have?** Choose a real
CLI/busprox example plus one retained dataset. That informs a useful inspector,
multi-bundle scenario, compatibility report, and manager publication design at
once. A read-only reporting prototype can proceed independently of the unfinished
manager coordinator.

For a broader direction, “named resources and recoverable state across process
lifetimes” is a credible center of gravity. It preserves the small independent
crates while giving the higher layers a compelling purpose. Kernel handover is
an exciting extension of that purpose, with its own evidence and compatibility
requirements.

## Cross-references

- [Design reading map](README.md): navigation back to the prior inquiry and
  implemented checkpoints; this assessment adds possible directions.
- [Architecture](architecture0.gpt6a.md#bundle-metadata-and-pluggable-state):
  foundation for names and opaque application schemas; broader recovery modes
  here extend its initial cooperative-only commitment.
- [Lifecycle checkpoint](lifecycle0.gpt6a.md#systemd-generation-decisions):
  persistent resources versus transient generations is an explicit existing seam.
- [Generation report](../../doc/systemd/generations0.gpt61s.md#upstream-deduplication-before-naming):
  concrete constraint on sharing unchanged resources across generations.
- [Direct report](../../doc/handover/direct0.gpt61s.md#real-process-example-and-evidence):
  completed implementation and single-bundle evidence, not an unimplemented
  universal multi-bundle scheduler.
- [Work map](work.md): existing implementation commitments; possibilities here
  need scope selection before becoming additional acceptance criteria.
