---
type: ArchitectureProposal
title: Systemd-native inventory and application-shaped adoption
description: Make semantic named resources and application schemas sufficient for simple recovery, including persistent mutable backing as a future application choice, with explicit completeness and authority where needed and manifests as an optional richer representation.
resource: /design/fd-handoff/systemd-native-adoption0.gpt6a.md
status: draft
generated: { by: "model:openai/gpt-6-astra#medium", at: 2026-10-14 }
sources:
  - resource: "opencode://ses_ec5b335ffffeIyNK21aCZL3jEw"
    title: "Commissioned design and mutable-state steering: audited contracts/systemd source; designed names-first adoption, optional completeness, persistent immutable or mutable custody, and proposed landing slices"
  - resource: "opencode://ses_ec616a6eaffe0I3U3HgN4tJ0Ho"
    title: "Parent commission and human direction: systemd inventory, application interpretation, optional manifest, and no implementation authorization"
  - resource: /design/fd-handoff/naming-opportunities0.gpt6a.md
    title: "Prior naming audit and transcript-grounded opportunities"
  - resource: /design/fd-handoff/architecture0.gpt6a.md
    title: "Capability decomposition and original manifest-centered higher layers"
  - resource: /design/fd-handoff/lifecycle0.gpt6a.md
    title: "Authority boundaries and transient ho1 generation decisions"
  - resource: /design/fd-handoff/work.md
    title: "Implementation checkpoints and parent-owned acceptance history"
  - resource: /doc/systemd/generations0.gpt61s.md
    title: "Current catalog mapping and manager deduplication constraints"
  - resource: /doc/systemd/custody0.glm53max.md
    title: "Offer/remove/barrier contracts"
  - resource: /doc/systemd/inspection0.gpt61s.md
    title: "Metadata observations and service incarnation limits"
  - resource: /doc/handover/core0.gpt61s.md
    title: "Cooperative authority sequencing independent of manifest values"
  - resource: /doc/handover/direct0.gpt61s.md
    title: "Implemented manifest delivery and offer-correlated direct control"
  - resource: /systemd/activation/src/lib.rs
    title: "Actual named lookup, ownership boundary, and duplicate handling"
  - resource: /systemd/fdstore/src/lib.rs
    title: "Actual FDNAME validation and non-atomic borrowed store/remove APIs"
  - resource: /systemd/generation/src/catalog.rs
    title: "Actual borrowed catalog and mandatory ho1 root mapping"
  - resource: /handles/bundle/src/lib.rs
    title: "Manifest associations, shared ResourceIds, and exact membership"
  - resource: /state/core/src/lib.rs
    title: "Format/version metadata presently outside opaque image bytes"
  - resource: /state/memfd/src/lib.rs
    title: "Independent sealed byte carrier and positional validation/read APIs"
  - resource: /handover/core/src/lib.rs
    title: "GenerationId dependency without a Manifest argument"
  - resource: /handover/direct/src/unix.rs
    title: "Current stage API binds manifest-bearing Offer to control correlation"
  - resource: /handle-off/Cargo.toml
    title: "Independent optional feature branches"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/man/systemd.service.xml"
    title: "Read locally: per-service store, metadata-only inspection, preserve policy, and LUO sessions"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/man/sd_notify.xml"
    title: "Read locally: name limits, distinct submissions, removal, polling, and LUO restrictions"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/docs/FILE_DESCRIPTOR_STORE.md"
    title: "Read locally: custody lifetime, soft reboot, kernel replacement, and manager forwarding"
  - resource: "https://github.com/systemd/systemd/blob/54b24a3855de37e5e19235c44acd113eb6ce6830/src/core/service.c"
    title: "Read locally: service_add_fd_store capacity and deduplication before name assignment"
---

# Systemd-native inventory, application-shaped adoption

**Let the application adopt named resources directly. Construct or transmit a
manifest when it adds information the application actually needs.** Systemd can
hold the physical inventory; application code can supply the schema that turns
`listener.http`, `cache.catalog`, or `session.83.socket` into meaningful roles.
A second serialized inventory is unnecessary for many useful applications.

This is a design proposal commissioned after the direct-handover checkpoint.
It selects no new acceptance criteria and authorizes no implementation. The
earlier multi-bundle aspiration does not imply a scheduler or selective-release
framework. The proposed change is in the center of the API story:

```text
inherited named handles + application schema -> validated application resources
                                                  |
                                                  +-> optional manifest/export
```

The simple path should be an example built from existing primitives first.
A reusable bounded name/group helper is justified if that example reveals
repeated mechanics. A schema DSL, universal recovery object, new wire protocol,
and digest-based commit format are not prerequisites.

## Current-state audit

| Existing capability | What is already possible | Relevant limit |
| --- | --- | --- |
| `activation` | Parse names, claim inherited owners once, `named`, `take_unique` | Names are tags; kind, access, compatibility and permission are application checks. `take_unique` preserves owners on missing/ambiguous selection. |
| `systemd-fdstore` | Offer any named borrowed descriptors; explicitly remove a name | One name per notification; all matching entries removed; no upsert or transaction. |
| `state-memfd` | Create/adopt immutable bytes independently of bundle/codec | Raw bytes have no toolkit format header. Current ceiling is 64 MiB; ownership-consuming adoption closes on failure, borrowed validation preserves originals. |
| `state` / `bundle` | Describe formats, extensions, arbitrary roles, aliases and shared ranges | `StateFormat` currently lives in extension metadata; arbitrary carrier bytes are not automatically self-describing. |
| `systemd-generation` | Borrowed `ho1` candidates and exact slot-to-manifest mapping | Root is mandatory for `map_manifest`; opaque slots cannot acquire semantic meaning by deleting it. Publication/retirement still unbuilt. |
| `handover` | Parked-owner authority bookkeeping and generation-correlated evidence | Depends on bundle's `GenerationId`, but takes no `Manifest`; callers attest actual quiescence, custody and release. |
| `handover-direct` | Dormant restore and explicit offer-correlated decision | `stage` takes the existing manifest-bearing transport `Offer`. Not an arbitrary named-inventory receiver. |
| `systemd-inspect` | Read names and kernel metadata, bracket an invocation in one snapshot | No arbitrary retained-FD download, root payload access, exact OFD proof or cross-step incarnation pin. |

Thus this is mostly a new application composition and documentation direction,
not removal of a foundational runtime requirement. Activation, fdstore and memfd
already have appropriately separate dependencies. The current direct transport
and `ho1` profile legitimately remain manifest-oriented.

## A simple path that stays simple

### Fixed roles and independent images

A small HTTP service declares in code:

| Name | Schema requirement | Startup action |
| --- | --- | --- |
| `listener.http` | Exactly one listening stream socket, expected address/family | Required in activated mode; validate before serving. |
| `cache.catalog` | Zero or one sealed image, supported catalog format and matching input revision | Use if valid; explicitly rebuild if absent or unusable. |
| `cache.routes` | Zero or one independent sealed image | Same rule, independently; not assumed coherent with catalog. |

No generation, root image, publication marker, bundle object or wire manifest is
needed. A socket unit can supply the listener with its configured descriptor
name; self-bound retained listeners can use fdstore. The application must choose
the source arrangement: receiving both under one singleton name is ambiguous.
If catalog and routes must describe one coherent cut, they are no longer
independent; use one combined sealed image or an explicit common checkpoint.

Proposed application code shape, **pseudocode rather than new library APIs**:

```rust
// Activation metadata has been parsed and exclusively claimed once at startup.
let http = require_one(activation.descriptors(), "listener.http")?;
validate_listener(http.handle.as_fd(), configured_endpoint)?;

let catalog = match optional_one(activation.descriptors(), "cache.catalog")? {
    None => build_catalog(inputs)?,
    Some(entry) => match validate_catalog(entry.handle.as_fd(), inputs.revision()) {
        Ok(image) => image,
        Err(reason) => rebuild_catalog_with_diagnostic(inputs, reason)?,
    },
};
// Transfer selected owners only after borrowed validation succeeds.
// Normal service startup authorizes serving; cache presence has no authority role.
```

`require_one`/`optional_one` can initially be ordinary application functions over
`Activation::named`/`descriptors`. Optional means absence is allowed; two matches
are still an error, including equal-looking handles. A configured discard/rebuild
policy can later handle a bad cache explicitly without guessing which duplicate
was intended. No manager mutation follows from validation failure.

For a retained image, use an application-owned bounded envelope, for example:
magic, format/version, header and payload lengths, dataset identity, input
revision, then payload. Define integer endianness and validate checked lengths
before allocating/decoding. Read positionally through the sealed-carrier API.
The complete image is the unit of validation; the name alone is not a version or
content check. Dataset identity and input revision prevent a perfectly valid but
irrelevant cache from being adopted. A digest is optional for content integrity
or cache keys; seals prevent mutation, not substitution or false application data.
An app-specific header avoids moving a redundant global manifest into every image.

### Discovered pairs

A schema can recognize `session.<id>.socket` and `session.<id>.state`. Enumerate
the union of IDs from **both** roles, then require exactly one of each for every
discovered ID. A state image names its session ID and capture ID; validate both,
its protocol version, socket class/access requirements, and sufficient buffered
state before dormant reconstruction.

This can describe several groups without a manifest. It says only that every
observed group is structurally complete. If both entries for session 84 vanished,
the discovery result cannot reveal session 84. Names-only is appropriate when
the application really means “recover whichever independent units are present”
and has a legitimate recovery/permission rule for each. It is not an exact
cooperative transfer of all sessions. Per-group authority is application-specific,
not created by naming a group or supplied by a new default scheduler.

For a cooperative bus-proxy cut preserving all live sessions, choose an expected
set or existing manifest; do not silently substitute this best-effort discovery.

## Semantic naming and scope

The recommendation is an **application semantic-name profile alongside `ho1`**,
initially an example convention rather than a toolkit-wide wire standard.
Use readable names directly where one application owns the service. Compose
prefixes only when multiple publishers deliberately share that service:

```text
listener.http
cache.catalog
session.83.socket
session.83.state

busprox.listener.http        # optional application prefix, distinct schema
busprox.session.83.socket
```

For the example profile, use dot-separated nonempty components matching
`[a-z0-9][a-z0-9_-]*`, whole name at most 255 ASCII bytes. Equality is exact;
do not lowercase, truncate, decode escapes, or normalize an input. A schema
defines its component positions and exact role vocabulary; generic splitting
does not invent containment. Fixed roles can remain stable as image format
versions evolve. Breaking name meaning requires a deliberate new application
profile/prefix, not treating old names as new by accident.

The toolkit's `FdName` remains broader (printable ASCII except `:`, including
empty names). The restrictive example is not a change to systemd or that API.
Arbitrary user IDs containing dots, Unicode or excessive lengths require an
application-issued bounded token with its native ID in state, or a separately
specified injective encoding. Never silently hash/truncate a full identity and
pretend no mapping or collision policy is needed.

The physical scope is `(manager instance/bus, canonical service unit, exact
FDNAME)`. Prefixes do not create another fdstore, quota, access boundary, or
retention policy. Persistent custody names span process invocations; an
InvocationID records an observation/publisher incarnation, not the lifetime of
the named resource. Dynamic tokens should not be reused while retained entries
or captures can refer to the old object. `83` is readable shorthand; a real
allocator must prevent ABA reuse across the chosen retention domain, for example
with a bounded random token or durable allocation epoch.

Names are exposed metadata: use application identifiers, not secrets or request
contents. Off-process tools can show names and observed metadata without claiming
to have validated image formats. Richer application inspection requires the
application to publish its own summary or inspect its inherited descriptors.

### Aliases, duplicates and bounded discovery

- Singleton roles reject repeated exact names even if metadata matches. No
  first/last winner and no deduplication by inode. A schema may explicitly declare
  a repeated listener role as a collection of equivalent endpoints, with separate
  suitability validation; removal still affects the entire named collection.
- One custody entry can serve two **declared application roles**. For example,
  a console resource can serve stdout and stderr in a fixed schema. Do not offer
  the same OFD twice under `stdout` and `stderr` and rely on both names appearing.
  Dynamic role-to-resource sharing belongs in app metadata or the richer manifest.
- Manager insertion checks capacity and deduplicates against the whole store
  before assigning the proposed name. Republishing an unchanged socket or image
  under another name can succeed as a submission and still create no new name.
  Independently opened files can also be conflated by weaker manager comparison
  paths; metadata equality is not an acceptable toolkit repair.
- Scan borrowed entries with bounded total entries, groups and name lengths;
  store sparse IDs in a map, never allocate to the numeric maximum ID. Preserve
  original indices for diagnostics. Discovery and validation do not claim or close
  FDs. Transfer owners explicitly after successful selection.
- Unnamed, unrelated and future-profile entries remain caller-owned. Report unknown
  roles inside the selected schema; default to rejecting them for exact adoption.
  An independent optional-resource app can deliberately ignore them. Malformed
  selected-profile names are errors, not absent groups. No implicit manager prune.

## What completeness information is actually needed?

Completeness is relative to an **expected set**, and that set needs a source.
The source can be compiled schema, deployment configuration, a caller's explicit
handover offer, app checkpoint state, or a full manifest. It need not be another
root FD. Separate four questions in ordinary APIs and diagnostics:

1. Which roles/groups can be found by name?
2. Are selected handles and images usable for the application's recovery mode?
3. What evidence establishes intended manager custody at the required cut?
4. Who has permission to operate after that cut?

These are questions to answer, not four compulsory new object hierarchies.

| Need | Smallest suitable representation | What it cannot establish alone |
| --- | --- | --- |
| Fixed listener and independent optional images | Application schema, image headers | Dynamic expected membership, old-owner relinquishment |
| Adopt every independently valid discovered pair | Pair grammar plus state correlation | A wholly missing pair or all-pairs commit |
| Exactly a known dynamic set at one cut | Explicit expected IDs/names in caller context or sealed app checkpoint | Custody causality, exact live-object identity, execution authority |
| Arbitrary bundle graph, shared resources/state ranges, generic transport | Existing full manifest and adapter binding | Resource validity or authority without their existing checks |

### The compact count/digest proposal

Do **not** introduce a standard generation/count/set-digest marker as the default.
It compresses a declaration, not the difficult protocol. Specific limitations:

- A count cannot distinguish `{83,84}` from `{83,85}`, or an old pair from a new
  pair. Specify whether it counts resources, groups, roles, or unique names.
- A digest can detect a set mismatch only with a known algorithm and unambiguous
  canonical input. A viable candidate would domain-separate profile/version,
  application scope and capture ID, then hash sorted length-prefixed tuples of
  exact name bytes, role and resource-revision binding. Reject duplicate names
  before hashing; define ordering and all lengths/integers byte-for-byte. Hashing
  an arbitrary JSON object or concatenated strings is not a specification.
- A name-set digest binds names, not contents, OFDs or immutable capture state.
  Include validated image revisions/content hashes where needed. A live socket
  has no universal immutable content digest; inode/flags are not exact OFD proof.
- The expected record must itself be found, bounded, decoded, correlated to the
  intended cut and selected without “largest generation wins.” A stale complete
  set plus its stale valid digest remains stale. If the record disappears entirely,
  absence cannot distinguish first start from a lost required handover.
- The store has no multi-entry atomic transaction. Publishing a record last does
  not prove the preceding offers were accepted, remain present, or belong to one
  immutable snapshot. Hashing mutable payloads adds read/write races rather than
  solving them. Freeze the application cut and use sealed capture images.
- Hashes do not authenticate a publisher or grant permission. A structural
  completeness record written before relinquishment cannot double as release
  authority. A commit record written afterward has a post-relinquishment failure
  window; its absence cannot authorize old resume.

For a modest dynamic inventory, **an explicit list is usually simpler**: put the
expected session IDs, exact custody names and image/capture bindings into an
already-needed sealed application checkpoint. It gives useful missing/unexpected
diagnostics and avoids a hash dependency and hidden canonicalization protocol.
This is a small manifest in the ordinary sense, but need not be the toolkit's
full bundle graph or a second list duplicating app state. A digest-only form is
worth choosing only after measurement or a concrete compact-commit use case.

Example proposed checkpoint, illustrative data rather than a selected wire format:

```text
busprox.checkpoint / format 1 / capture k7 / recovery exact-cooperative
  expected:
    session.a12.socket -> state image session.a12.state.k7
    session.b09.socket -> state image session.b09.state.k7
  required shared catalog: cache.catalog.r4
```

The socket names are stable custody identities; the state images bind the capture.
Both missing members of `b09` are now detectable. The publisher must bind those
names to the intended live resources, use exclusive publication ownership and
retain inspection evidence; the checkpoint cannot prove that binding by itself.
Fresh checkpoint/image objects prevent same-OFD retagging. A cooperative release
decision, if persisted, additionally binds the selected checkpoint/capture and
ownership domain under a specified protocol. This document does not define that
manager release protocol; the existing lifecycle requirements still apply.

## Custody lifetime, updates and retirement

Persistent custody and prepared captures have different clocks. A listener or
catalog can remain stored across many process replacements without being
re-offered or renamed. A transient capture references that existing custody;
its retirement must not remove persistent entries. This differs from `ho1` v1,
whose resource slots belong to the selected transient generation.

### Mutable backing is a different, important use of that lifetime

The operator also raised mutable retained state, with a memfd-backed SQLite
database reached through `/proc/self/fd/N` as an exploratory illustration. This
makes the motivation stronger: **a named resource can be the application's
long-lived working state, not merely a package sent to its replacement**.
The selected architecture need not serialize an entire world at every handover.

For example, `state.catalog` could name an application-owned mutable backing
resource, while `cache.catalog` remains an independently replaceable derived
image. Names express roles, not a universal durability or consistency guarantee.
If `state.catalog` is authoritative, missing or corrupt state is an error requiring
the application's recovery policy; the cache example's rebuild fallback cannot
be copied blindly. A manifest remains optional when the application's known
schema and backing format contain all required associations.

Distinguish three useful cases:

| Resource | What crosses process replacement | What makes adoption legitimate |
| --- | --- | --- |
| Sealed transport snapshot | An immutable capture at a named cut | Validate image and exact required resources; correlate release for live continuation. |
| Mutable backing, quiescent handover | The same retained store, after old writers stop at an application-defined consistent boundary | Validate/recover its committed root or transactions; relinquish old access and grant new ownership. A small capture may still hold process-local stream state. |
| Mutable backing during live operation/crash | Ongoing application state, possibly interrupted mid-update | The backing format's actual interruption recovery and concurrency rules; fdstore supplies references, not transactions. |

A mapping address, Rust pointer, mutex owner or runtime object from the old
process is not a portable persisted reference. A shared arena needs offsets or
another reconstructible representation and appropriate process-death handling.
Reopening a format must not depend on the inherited FD retaining the old number.
Writable aliases require explicit coordination; holding another FD is neither a
writer lock nor revocation of the old actor. Checks that are safe on immutable
images can race on mutable bytes, so validate at a stable boundary or use the
format's supported concurrent-read protocol.

Crash consistency is application/format work: transactions, a journal, committed
roots, or another suitable method. Keeping bytes alive does not make interrupted
multiword writes atomic. A transaction in one backing resource does not commit
two datasets, buffered userspace state and socket output together. A handover may
still need to park writers, finish/abort transactions, capture pending I/O, and
record which database revision that capture corresponds to. Even a database with
correct transaction recovery cannot retract bytes already sent to a peer.
Mutable backing is not automatically arbitrary-crash stream continuity.

The SQLite thought experiment deserves a small, separate feasibility probe if
selected. `/proc/self/fd/N` gives a process-local path to an FD; it does **not** by
itself establish that SQLite's ordinary filesystem VFS works correctly there.
Before claiming support, check:

- path canonicalization and reopening behavior, including whether SQLite opens
  another description or resolves a memfd label that cannot be reopened;
- journal, WAL and SHM naming/opening, temporary files and their retention paths;
- locking and shared-memory behavior across intended processes and connection
  lifetimes, including inherited references that can prolong lock/resource life;
- a fresh successor's changed descriptor number and stable rediscovery of every
  required backing component, plus crash recovery for the chosen journal mode.

Choosing in-memory journals or a custom VFS changes the recovery contract; neither
is an automatic fix. A normal filesystem-backed database could be a simpler
application choice. No database dependency or VFS work is selected here.
Retained memfd state is volatile across unsupported transitions and power loss;
process survival, crash consistency and durable storage are separate properties.

The existing `state-memfd` API intentionally requires write seals and rejects
mutable backing. Keep that guarantee. An app can manage its mutable FD using
ordinary OS APIs and the existing activation/fdstore primitives; introduce a
separate backing helper only for demonstrated shared needs. Do not weaken sealed
image validation or add database knowledge to the carrier or named inventory.
Any compact integrity digest refers to a frozen revision, not a promise that
live mutable bytes remain equal to a past hash. LUO support for a chosen mutable
backing and its recovery semantics needs explicit evidence, not inference from
the successful immutable-image path.

### Inserting and replacing

There is no `put(name, fd)` replacement operation. Offering a second, different
resource under an existing name creates ambiguity; removing that name then removes
**both**, not only the old one. Practical choices should remain explicit:

1. **Independent rebuildable cache:** remove the old stable name, verify absence,
   then offer a fresh image under it. Keep local references until verification.
   A restart in the gap rebuilds. This is a deliberately lossy cache update, not
   an atomic handover, and is often the simplest correct application contract.
2. **Retain either valid cache revision:** offer a fresh image under a distinct
   revision name such as `cache.catalog.r4`. The schema selects any image matching
   the required input revision, or applies an explicit version policy. Stale
   valid images are not automatically current; ordering opaque tokens proves
   nothing. Retire named old revisions deliberately when no chosen consumer needs
   them. This costs extra slots but needs no root when any valid choice suffices.
3. **Exact coordinated replacement:** retain old/new versioned objects and use an
   explicit checkpoint/selection protocol. Resolve crash windows, old selection
   invalidation and execution authority before calling it committed. Prefer the
   established manifest path where it fits rather than inventing this protocol
   merely to avoid a manifest.

Never emulate rename by re-offering the same OFD under a new name. Remove/re-add
with the original retained locally has a manager-custody gap; a crash there loses
the manager reference. For an existing persistent socket, keep the custody name
and correlate new state through capture metadata instead. The application must
prevent another publisher rebinding that name during verification and use.

### Publication evidence and failure

Retain the intended plan, originals and all possibly submitted names before
sending. Different names require separate notifications. A send error can follow
delivery; a barrier provides ordering, not acceptance. Capacity observations are
not reservations. When custody is correctness-critical, inspect the intended
canonical unit and preserve MainPID/InvocationID evidence across steps, allowing
the deliberate old-to-new invocation transition rather than mistaking it for
one unchanging lifetime. Existing inspector convenience verification does not
do this transaction-level tracking. Choose resource-specific guarantees adequate
to the application; do not turn metadata membership into exact OFD certification.

For optional caches, unverified retention may merely mean the next instance
rebuilds; report that honestly. For a continuity attempt, incomplete or ambiguous
custody prevents relinquishment. After irreversible relinquishment, missing
release/commit evidence is a recovery failure, not permission for the old owner
to resume. `READY=`, `STATUS=`, watchdog progress and `STOPPING=` cannot repair it.

Default manager polling may drop HUP/ERR resources; disabling it with `FDPOLL=0`
can retain application-relevant state but also pins dead references. Choose
deliberately per workload. Removal is exact-name, all-matches and explicit.
Verify required removal separately; manager absence does not prove application
aliases closed. Closing the local copy alone may leave peers pinned by fdstore.
Unknown entries and failed attempts are never automatically pruned or reconciled.

Configuration is part of custody: one per-service descriptor quota, NotifyAccess,
retention policy and any enveloping manager capacity. A named prefix supplies none
of these. `restart` and separate stop/start are different; `Preserve=yes` has a
different lifetime from `on-success` or the default. The checked development
manpage and overview differ in some retention wording, so deployment examples
must target a tested manager version rather than promise universal behavior.

## Recovery and permission across different transitions

| Transition | Suitable interpretation |
| --- | --- |
| Ordinary activated startup/restart | Validate required roles; rebuild explicitly optional caches. Startup supervision may establish the new service's ownership, with actual prior actors terminated. Cache presence need not encode a handover lease. |
| Cooperative session continuation | Validate a sufficient parked capture and its exact required inventory; reconstruct dormant, then honor the separately correlated relinquishment/release decision. No discovery-based partial success. |
| Crash restart | A surviving socket and an old image do not imply consistent stream offsets or pending writes. Permit only application-defined crash-recoverable checkpoints; otherwise reject continuation or explicitly reconnect/rebuild. |
| Same-kernel soft reboot | Custody can survive with suitable unit/manager configuration, but application state sufficiency and permission still need validation. Merely renaming resources does not establish pass-through. |
| LUO kernel replacement | The checked systemd development documentation preserves supported objects (currently memfds) and drops unsupported types such as sockets. Validate the actual inherited inventory and image compatibility after restoration. |

If a committed session checkpoint survives LUO while its sockets disappear,
**reject exact handover as incomplete**. If socket and state both disappear, an
expected set still detects the missing group. A separate recovery mode may use
surviving catalogs and establish fresh connections; report that as a different
recovery, never successful partial committed continuity. Even total loss of the
expected-set record must not silently become cold start when the launch expected
a handover: carry that expectation through the launcher/deployment or another
trusted control source. The store alone cannot distinguish every total-loss case.

Multiple `LUOSession=` names represent actual kernel preservation sessions, not
extra fdstores or an application grouping solution. Kernel compatibility and
restoration tests remain specific to the source/target pair and resource type.

## API and dependency shape

Recommended progression:

- **First use existing APIs.** Small applications use activation and, if they
  retain resources, fdstore/notify. Add memfd only for immutable images and choose
  the application's codec. No required `bundle`, rkyv, inspector, runtime, or
  coordinator for the fixed-role example.
- **Extract mechanics only after use.** A proposed optional `systemd/named`
  helper could borrow `(Option<&str>, &H)` entries, offer unique/optional/many
  selection, and bounded grouping via an application parser. It returns borrowed
  selections and diagnostics; it does not choose a schema, own FDs, mutate manager
  state, select newest, or issue permission. Keep it std-only if extracted.
  Do not depend on `systemd-generation` just to reuse its tiny `NamedHandle` type.
- **Validation stays callable application code.** A Rust function describing
  catalog compatibility or paired session recovery is an adequate schema.
  Typed application structs hold successful selections. Avoid a universal trait
  lattice or declarative registry before a second consumer establishes the need.
  A mutable backing's transaction/open/recovery API stays with that application
  or storage library, outside name selection and descriptor custody.
- **Optional projection goes downstream.** A manifest builder can describe a
  validated application's resources/aliases/ranges for generic transport or
  inspection. It needs an explicit capture/generation when one is meaningful;
  don't fabricate a permanent handover generation for an ordinary cache view.
  Revalidation is required if the application cut changes before export.

The core coordinator's foundation-only `GenerationId` dependency is not a wire
manifest dependency. Keep it for now; relocate shared identity vocabulary only
if real reuse warrants the churn. Current direct control remains tied to its
manifest-bearing transport. A future names-oriented direct format would require
its own association and bounded wire specification; it is not needed to make
the systemd path simpler.

## Coexistence and where richer manifests pay off

`ho1` is a manifest-backed transient-generation profile, not a malformed older
version of semantic names. Preserve its exact slot/root/marker rules. A semantic
schema owns a disjoint name family and treats `ho1` entries as unrelated. An app
that supports both explicitly selects a profile; it does not merge matching
logical IDs across them or fall back after failed committed-generation validation.

The full manifest earns its place for an invocation whose stdout and stderr share
one resource, several connections whose buffered state occupies ranges of one
sealed image, or a generic direct recipient that does not know the sender's schema.
For example:

```text
resources: client-a, client-b, shared-output, capture-image
bundles:
  invocation-17: stdout -> shared-output; stderr -> shared-output
  session-a: connection -> client-a; writer -> capture-image[0..128]
  session-b: connection -> client-b; writer -> capture-image[128..384]
```

Here explicit graph/range information prevents duplicating descriptors and state
images and makes transport interoperable. It is useful representation, not a
mandatory registration system through which the HTTP listener must pass.
The current manifest cannot bind `ho1` slots directly to arbitrary stable external
custody names; that needs a future binding/profile change, not a reinterpretation
of current validation. Coexistence also does not let one OFD occupy both profiles:
manager deduplication may suppress the second tag. Share through an explicitly
designed binding or use distinct resources, never assume dual publication works.

## Proposed landing slices and meaningful evidence

These are decision-ready slices, not new tickets or implementation assignments.

1. **Names-first fixed-role example and documentation.** One listener plus an
   independently retained self-describing cache, including empty first start and
   explicit rebuild. Measure dependencies; show no bundle/codec/coordinator is
   pulled in unless selected. Review whether actual code needs any new helper.
2. **Bounded selection/grouping, if repetition warrants it.** Preserve owners;
   exercise shuffled delivery, unrelated entries, duplicate singleton names,
   optional absence, malformed selected names, orphan roles and sparse large IDs.
   Explicitly demonstrate that a wholly missing pair is unknowable without an
   expected set. Avoid tests that merely mirror parser branches.
3. **Persistent cache update under a real manager.** Test the selected lossy or
   versioned policy across repeated service invocations, interruption between
   remove/offer, capacity exhaustion, and explicit retirement. Verify retained
   image identity/bytes and listener continuity, not just matching tag counts.
   Test same-OFD retag suppression and duplicate-name all-match removal on the
   actual manager/kernel capability combination.
4. **A concrete exact session capture, only when commissioned.** Decide whether
   the app's own checkpoint or existing manifest is smaller in total complexity.
   Fault-inject every publication boundary, missing entire groups, stale images,
   name reuse, altered incarnation, lost release and duplicate candidates. A
   pending-write process demonstration proves no conflicting I/O while dormant
   and exact continuation after authority transfer. Keep it one coherent release
   unless the application independently justifies finer ownership domains.
5. **Optional format/adapter extraction.** Generalize only the demonstrated
   shared mechanism. Systemd conformance, soft reboot and LUO need separate
   isolated deployment evidence; synthetic missing-socket tests are useful but
   do not establish actual kernel preservation support.

A mutable-backing feasibility probe is an independent optional branch, not a
prerequisite for those slices: choose one format and one ownership/recovery rule,
exercise orderly replacement and interrupted mutation, and verify state rather
than just retained bytes. For the SQLite illustration, test sidecar/reopen/locking
behavior and fresh-process FD renumbering before designing any reusable adapter.
That investigation can conclude the ordinary VFS/path approach is unsuitable.

## Alternatives and decisions for the operator

**Recommended:** application naming conventions and executable schema examples
first, existing manifest path for graph-rich/exact generic transfer. This gives
immediate ergonomics without creating another mandatory publication protocol.

Alternatives remain reasonable in their own scopes:

- Always manifest: uniform generic tools and one representation, but redundant
  root/codec/slot machinery for known roles and independent images.
- Put all metadata in names: externally readable, but the bounded grammar becomes
  an awkward schema, aliases are hard, and no completeness/authority problem is
  solved. Keep payload details in self-describing state.
- Compact digest root: fewer metadata bytes for very large sets, but worse
  diagnostics and a canonicalization/correlation protocol still to maintain.
- One packed self-describing app image: excellent for mutually coherent small
  datasets, but cannot package arbitrary live sockets and makes independent
  dataset replacement coarser. Choose based on actual state relationships.

Consequential choices before implementation are: which first real application
schema to demonstrate; whether cache update gaps/rebuild are acceptable; whether
dynamic sessions require exact all-session continuity or independent recovery;
where the expected handover/capture selection comes from across restart; and
whether to commission a separate mutable-backing feasibility probe.
The first example can proceed after selection without resolving a universal
semantic-name wire standard, manager transaction protocol, or selective scheduler.

## Cross-references and verification

- [Reading map](README.md): reciprocal navigation for this design.
- [Prior naming assessment](naming-opportunities0.gpt6a.md#6-opportunity-persistent-named-state-alongside-transient-handovers):
  motivation for persistent resource lifetimes. This proposal changes its default
  hierarchy from logical names above opaque addresses to direct semantic tags
  where the application schema is sufficient.
- [Architecture](architecture0.gpt6a.md#bundle-metadata-and-pluggable-state):
  retains meaningful codec/carrier decomposition while making its richer metadata
  one optional route through higher-level use.
- [Lifecycle](lifecycle0.gpt6a.md#systemd-generation-decisions) and
  [generation report](../../doc/systemd/generations0.gpt61s.md):
  existing transient-profile obligations, not rules silently deleted by this draft.
- [Custody](../../doc/systemd/custody0.glm53max.md) and
  [inspection](../../doc/systemd/inspection0.gpt61s.md): actual mechanisms and
  evidence limits behind the proposed application examples.
- [Core](../../doc/handover/core0.gpt61s.md) and
  [direct](../../doc/handover/direct0.gpt61s.md): reusable authority distinctions
  and the implemented richer transfer route.
- [Work map](work.md): native implementation/acceptance history remains with its
  owner; these proposed slices do not modify that history.

Verification for this design: read the cited local reports, implementation paths
and archive sources; confirmed the archive revision shown in the source URLs;
searched surrounding documentation for schema, persistent-state and adoption
relationships; checked local Markdown links. This is source/design review, not
compiled API examples, a new test run, live-manager mutation or LUO conformance.
