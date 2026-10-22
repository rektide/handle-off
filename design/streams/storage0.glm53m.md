---
type: Research
title: Durable Streams storage representations and handle-off mapping questions
description: Source-inspected Caddy and TypeScript storage layouts, framing, metadata, forks and retention, with bounded implications for resource sharing.
resource: /design/streams/storage0.glm53m.md
status: draft
generated: { by: "model:openai/gpt-6-astra#medium", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9aadbe9dffexC59pP63OGevjS"
    title: Researcher inspected upstream storage, handle-off identities and shared-reader scope; preserved findings in the parent-requested filename
  - resource: "opencode://ses_e9afd800bffeVHLHIvV9eCKpzQ"
    title: Parent commission to preserve researcher-written storage findings and separate grant work from storage tickets
  - resource: "https://github.com/durable-streams/durable-streams/tree/461b40267aabd644558f9b19dbb9507dd5f691cf"
    title: Durable Streams checkout revision inspected for this research
  - resource: /design/fd-handoff/naming-opportunities0.gpt6a.md
    title: Existing scoped names and retained immutable versus mutable state distinctions
  - resource: /design/fd-handoff/lifecycle0.gpt6a.md
    title: Existing custody and execution authority boundaries
  - resource: /README.md
    title: Existing resource, carrier, delivery and rendezvous capabilities
  - resource: "https://www.rfc-editor.org/rfc/rfc9562.html#section-4"
    title: Standard UUID URN representation
  - resource: "https://man7.org/linux/man-pages/man2/pidfd_open.2.html"
    title: pidfd identity and process-exit observation semantics
---

# Existing storage before new abstractions

**Caddy currently has one appendable `data.seg` per stream. It does not have a
rotating collection of immutable segments.** Forks reference ancestor streams and
add their own file. The TypeScript implementation uses a different framing and
metadata backend; the memory implementations are different again.

This records the research already returned to the parent, rather than a new
implementation proposal. The user requested this exact filename; its `glm53m`
suffix is not an authorship claim. The researcher was GPT-6 Astra medium, as
recorded in the metadata and native session source.

## Evidence and scope

Inspected `/home/rektide/archive/durable-streams/durable-streams` at
`461b40267aabd644558f9b19dbb9507dd5f691cf`, confirmed with `git rev-parse HEAD`.
The links below bind claims to that revision. Findings are **source inspection**;
no upstream tests, crash experiments, or benchmarks were run. Apparent recovery
problems are observations needing focused verification, not reproduced failures.

Current work anchors are `handoff-streams-storage`, `handoff-streams-model`, and
`handoff-streams-forks`, per the parent. Grants, URI identities, deadlines and
leases are separately scoped. This document neither implements those capabilities
nor changes tickets. Independent journal and durable-workbench inquiry remains
with its commissioned researchers.

## Caddy file store

### Files and the meaning of segment

```text
<dataDir>/
  metadata/
    metadata.db                    shared bbolt database
      bucket "metadata":
        stream path -> JSON metadata, including directory_name
  streams/
    <escaped-path>~<timestamp>~<random>/
      data.seg                     this stream's own append records
    <fork-path>~<timestamp>~<random>/
      data.seg                     fork's own records
```

One stream has one segment file, including an empty file on creation. A fork
creates its own file without copying the entire parent. Metadata is not a
per-stream sidecar: the database is shared. Stream directories use URL-escaped
paths, a nanosecond timestamp and a four-random-byte hex suffix. These directory
names distinguish physical instances from reusable logical paths.

Sources: [segment constants and framing](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/segment.go#L12-L29),
[stream directory/file creation](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L283-L301),
[directory naming](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L1339-L1355),
[bbolt location and bucket](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/bbolt.go#L60-L80).

### Records, content boundaries and offsets

Each record is `[4-byte big-endian payload length][payload bytes]`, without a
newline delimiter. The framing contains no checksum, timestamp, producer ID or
stream metadata. A payload is limited to 64 MiB.

- Binary/non-JSON: one append becomes one record.
- JSON: a top-level array is flattened one level into separate records; a
  non-array value becomes one record. Nested arrays remain values.
- HTTP formatting removes storage framing: binary payloads concatenate and JSON
  messages become an array.

```text
append "abc":
  physical bytes [0,7)   = [length 3][abc]
  returned offset        0000000000000000_0000000000000007
append "de":
  physical bytes [7,13)  = [length 2][de]
  returned offset        0000000000000000_0000000000000013

HTTP payload: abcde      = 5 content bytes, not 13
```

For an ordinary file-backed stream, `ByteOffset` is the physical position used
for seeking and includes length prefixes. `ReadSeq` is normally zero; its comment
reserves it for future rotation. The `offset.go` comment saying byte offsets
exclude framing disagrees with executable file-store code. Do not use that
comment as a storage contract.

Sources: [record write/read](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/segment.go#L39-L80),
[seek and returned offsets](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/segment.go#L104-L150),
[actual append accounting](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L797-L843),
[JSON flattening](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/memory_store.go#L1038-L1068),
[offset declaration](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/offset.go#L9-L35),
[HTTP response formatting](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L1318-L1337).

Protocol clients must still treat offsets as opaque. A physical byte mapping is
an implementation detail, not a cross-backend API guarantee. The
[protocol's offset section](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/PROTOCOL.md#L1186-L1238)
explicitly permits implementation-defined encodings.

### Metadata inventory

| Concern | Fields |
| --- | --- |
| Identity and representation | Path, ContentType, CurrentOffset; persisted DirectoryName |
| Ordering | LastSeq (`Stream-Seq`) |
| Lifetime | TTLSeconds, ExpiresAt, CreatedAt, in-memory LastAccessedAt |
| Idempotent producers | Map from producer ID to Epoch, LastSeq, LastUpdated |
| Closure | Closed, ClosedBy producer ID/epoch/sequence |
| Fork relation | ForkedFrom, internal ForkOffset, ForkOffsetRequested, ForkSubOffset |
| Retention | RefCount of child forks, SoftDeleted |

The bbolt `metadata` bucket maps stream path to JSON serialization of these
fields, with an important exception: **LastAccessedAt is not serialized or
restored at this revision**. The runtime uses it to calculate sliding expiry.
No reader-grant ledger is represented by `RefCount`: it counts fork references.

Sources: [runtime metadata and expiry](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/store.go#L196-L235),
[serialized metadata](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/bbolt.go#L22-L58),
[serialization and path key](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/bbolt.go#L95-L160),
[cache reconstruction](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/bbolt.go#L433-L513).

### Append and read operation

`Append` takes the store-wide metadata mutex, validates producer/sequence state,
writes records and syncs the file, then updates memory and bbolt metadata. Some
metadata-write errors are ignored after file write. File data and metadata are
not one transaction; framing alone cannot reconstruct producer or closure state.

`Read` holds the same exclusive metadata mutex while reading the complete
ancestor chain, preventing delete/recreate from changing the selected metadata
generation. It opens a fresh segment reader, seeks and decodes to EOF. Multiple
clients have independent logical cursors, but these store reads are serialized
under that mutex. This is not an exposed shared-FD reader protocol.

Sources: [Append](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L641-L843),
[fresh reader](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L886-L901),
[generation-pinned Read](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L977-L1026).

### Fork mapping

```text
parent P: data.seg = [abc][de]        framed ends: 7, 13
child C: forked_from=P, fork_offset=7
         data.seg = [XY]             physical end: 6

C logical content: abcXY
C logical record ends: 7, 13
C local physical byte 0 <-> C logical byte 7
```

The child reads ancestor messages up to its fork boundary, followed by its own
segment. Later parent appends are excluded. The child-local physical offset is
logical byte offset minus the fork base; returned local offsets add the base.
Nested forks recursively traverse the chain. Coincident numeric offsets in two
branches do not identify identical content.

A binary sub-offset copies a prefix of the next source message into the child's
file. JSON sub-offsets count flattened messages; Caddy advances the internal fork
boundary to a source message boundary while retaining requested values for
idempotent creation matching. This is not full-file copy-on-write or a hard link.

Sources: [fork creation and reference acquisition](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L190-L258),
[binary prefix materialization](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L314-L350),
[sub-offset resolution and recursive read mapping](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L845-L975).

### Retention and recovery boundaries

Implemented lifetime controls include whole-stream TTL, absolute expiry and
deletion. A parent with forks is soft-deleted and retained for inherited reads;
its path cannot simply be reused while pinned. Deleting a child decrements its
parent edge and can cascade collection of soft-deleted ancestors. Startup
reconciles reference counts from child metadata. This is distinct from active
reader accounting.

No segment rotation or age-based prefix trimming was found in the inspected
Caddy store implementation. The protocol's statement that servers *may*
implement retention does not establish either capability here.

Recovery scans complete frames and truncates a damaged tail. Two source-level
concerns qualify reuse of that machinery:

1. Last-access time is absent from persisted metadata, so the actual sliding-TTL
   window is not recovered by the inspected reconstruction path.
2. `RecoverStoreWithEvents` reconciles CurrentOffset directly to scanned physical
   length without visibly adding a fork's logical base. Fork restart behavior
   needs focused verification before relying on this as correct logical recovery.

Sources: [delete and cascade](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L442-L531),
[reference reconciliation](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L118-L159),
[expiry cleanup](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L1275-L1314),
[recovery](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/file_store.go#L1374-L1448).

## Memory and TypeScript representations

### Caddy MemoryStore

```text
map[path] -> memoryStream {
  metadata: StreamMetadata
  messages: []Message { Data: []byte, Offset }
  data: []byte                    additional accumulated non-JSON bytes
}
```

There are no segment files or stored length prefixes. Offsets advance by payload
length only: `abc` then `de` yields 3 and 5, whereas Caddy FileStore yields 7 and
13. JSON appends use the same one-level flattening but no framing overhead.
Reads select messages whose end offsets exceed the requested offset. Forks follow
ancestor message slices capped at their boundaries; metadata/refcounts remain in
process memory. These are not mmap-backed arenas or memfds.

Sources: [memory structure](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/memory_store.go#L12-L28),
[append, fork and read](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/caddy-plugin/store/memory_store.go#L718-L875).

### TypeScript reference server

Its file store has one `streams/<unique-stem>.log` and shared `metadata.lmdb`.
The append framing is **four-byte big-endian length, processed append payload,
one newline**. Offsets include five framing bytes per stored append. JSON
processing retains processed append batches; Caddy's per-element file framing
must not be assumed here.

Metadata includes path/content type, current offset, sequence, TTL/expiry,
creation and last-access times, directory name, segmentCount, totalBytes,
producer state, closure state and fork/refcount/soft-delete fields. Unlike the
inspected Go serialization, TypeScript explicitly includes last-access time.
`segmentCount` is initialized to 1; that field alone is not evidence of rotation.

The TypeScript memory store is a Map of stream objects containing message arrays.
It stores `{data, offset, timestamp}` per append and advances offsets by the same
five-byte framing overhead even though it has no framed file. It also implements
sliding expiry and fork-reference retention. Thus even the two memory backends
do not share one physical-offset interpretation.

Sources: [TS persisted metadata](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/server/src/file-store.ts#L38-L105),
[file path and metadata initialization](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/server/src/file-store.ts#L266-L319),
[initial segment count](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/server/src/file-store.ts#L948-L970),
[file append framing](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/server/src/file-store.ts#L1310-L1367),
[memory map and expiry](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/server/src/store.ts#L166-L229),
[memory append accounting](https://github.com/durable-streams/durable-streams/blob/461b40267aabd644558f9b19dbb9507dd5f691cf/packages/server/src/store.ts#L1350-L1383).

## Mapping into existing handle-off resources

Today handle-off offers logical ResourceId associations, bundle members sharing
an ID, opaque versioned state extensions, and positional resource ranges. Its
sealed memfd carrier produces immutable images; duplicated descriptors retain
shared OFD offsets/status flags. These facilities do not define a stream store,
fork graph, retention index or durable producer ledger.

Existing sources: [resource table](/handles/core/src/lib.rs),
[bundle associations](/handles/bundle/src/lib.rs),
[inline and resource-range payloads](/state/core/src/lib.rs),
[sealed carrier](/state/memfd/src/lib.rs), and
[retained-state assessment](/design/fd-handoff/naming-opportunities0.gpt6a.md#6-opportunity-persistent-named-state-alongside-transient-handovers).

The storage/model/forks work needs explicit answers to these questions:

1. Is the exported object raw backend framing, decoded payload records, or a
   new snapshot format? Record that choice and its version; offsets cannot be
   blindly copied between the four representations above.
2. Which metadata accompanies bytes: exact committed tail, content type,
   incarnation, fork boundaries, producer and closure state? A file FD alone is
   insufficient to reconstruct the stream's meaning.
3. Does a snapshot materialize inherited content or retain an ancestor graph?
   The latter needs retained ancestor identities, ranges and lifetime obligations.
4. If sharing a growing file, how is the visible prefix bounded and coordinated
   with metadata? Upstream's mutex is process-local. A sealed image avoids writes
   but requires an explicit export operation; it is not the existing live file.
5. How does a logical stream path distinguish delete/recreate incarnations?
   A reusable name and an exact snapshot identity serve different purposes.
6. Which crash guarantees are required? Framed data recovery cannot recreate
   all separate metadata, and the inspected recovery concerns need verification.

Several sealed memfds might eventually be a useful representation, but calling
them upstream segments would conceal a new partitioning/export design.

## Preserved grant and identity findings — separate scope

The earlier recommendation was an optional shared-immutable-image grant facility,
composing existing bundles/carriers rather than changing writer handover. Its
proposed ticket was not created. Current parent direction separates this from
the three streams storage/model/forks tickets.

### What identifiers already mean

ResourceId, BundleId, MemberId and GenerationId accept exact nonempty strings
without controls. URI-shaped strings are possible, but there is no built-in URI
identity, canonicalization or dereferencing. Rendezvous accepts filesystem paths
and `@abstract` Unix addresses and returns unverified endpoint candidates.
Generation `ho1.*` names are backend tags. See
[existing naming assessment](/design/fd-handoff/naming-opportunities0.gpt6a.md#1-what-we-name-today),
[rendezvous contract](/rendezvous/core/src/lib.rs), and
[endpoint parsing](/rendezvous/core/src/unix.rs).

`urn:uuid:<uuid>` is standardized by RFC 9562 and can identify a freshly assigned
grant or participant incarnation. It provides no discovery or authority.
Readable forms such as `handle-off://<scope>/resource/<token>` or
`handle-off://<scope>/grant/<token>` were illustrative private conventions, not
existing or registered handle-off schemes. Structured scope/incarnation fields
remain a viable baseline.

PID is reusable and namespace-relative. Process identity needs incarnation
evidence; pidfds help bind a live process and observe exit, but acquiring one
from a stale numeric PID still needs validation. A systemd unit name identifies
the continuing unit; InvocationID identifies one runtime cycle and is shared by
that unit's processes. A job ID identifies a manager operation, not the service
incarnation or a worker. These conclusions were checked against local systemd
`man/systemd.exec.xml:4246–4252` and
`man/org.freedesktop.systemd1.xml:12810–12823`, plus the pidfd manual linked in
sources. The systemd checkout revision was not captured in that earlier check;
these are file/line observations, not revision-bound systemd evidence.

### Grant boundaries worth retaining

- Identify issuer and holder incarnations, exact resource/version/range, rights,
  grant ID and agreed lifetime. Read grants and exclusive writer authority differ.
- Completion-of-use and optional agreed duration can end retention obligations;
  define deadline start/clock, early release and restart behavior explicitly.
- Custody ACK is not a grant. Expiry, completion or deleting a metadata value
  cannot revoke duplicated FDs or mappings. Closing the issuer's FD does not
  invalidate another holder's reference.
- A reader's death does not prove delegated copies are gone. Issuer restart with
  missing grant records is not proof that all grants ended.
- A resolver should bind exact incarnations, with stale/absent/ambiguous outcomes,
  rather than silently selecting a replacement process or newest resource.
- Small `-read`/`-write` values or metadata memfds can describe claims; their names
  are not locks or proof of authority. Multiple readers need distinct grant
  records or an explicit collection. A sealed metadata image cannot mutate itself
  when a grant ends.

These are future grant-design constraints, not acceptance criteria silently added
to storage work. Existing [handover authority boundaries](/design/fd-handoff/lifecycle0.gpt6a.md#separate-custody-from-permission-to-operate)
remain relevant precedent, while the upstream fork reference count supplies a
specific retention mechanism rather than a ready-made reader lease system.
