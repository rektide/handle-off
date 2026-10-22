---
type: ReviewAddendum
title: Second-stage journal integration and stream routing assessment
description: Independent review of the delivered Durable Streams reader seam, source-checked journal precedents, and concrete next journal fixture and reference work.
resource: /design/streams/integration0.gpt61s.md
status: draft
generated: { by: "model:openai/gpt-6.1-sol#xhigh", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a25db9cffeW8IeTiVrCNzdoC"
    title: Explicitly selected Sol xhigh independently reviews delivered Durable Streams APIs and journal precedents; prepares the second-stage integration assessment without reading the unfinished journal implementation
  - resource: /design/streams/durable-workbench0.gpt61s.md
    title: Delivered workbench account, attributed checks and real server observations
  - resource: /stream/durable-streams/src/lib.rs
    title: Actual HTTP batch, source binding, file reader and output-before-save APIs
  - resource: /stream/durable-streams/tests/reader.rs
    title: Inspected deterministic negative controls and save-after-output replay test
  - resource: /stream/durable-streams/fixtures/live.py
    title: Actual installed-binary retained attempts and server restart fixture
  - resource: /design/streams/journal-prior0.gpt6l.md
    title: Recovered earlier journal research and scoped reader precedents
  - resource: /design/streams/nodeon0.gpt6l.md
    title: Caller-owned inputs, immutable evidence versus checkpoint persistence, and source routing
  - resource: /design/streams/nodeon-review0.gpt6l.md
    title: Independent review of report-only navigation and negative-control requirements
  - resource: /design/streams/storage0.glm53m.md
    title: Backend-specific framing, logical offsets and separate retention authority
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Existing locator, kind, explicit connection and relative-reference vocabulary
  - resource: "file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md"
    title: Prior journal survey read to recover Cyclrs and Phyllited mechanics and limits
  - resource: "file:///home/rektide/src/cyclrs/source/journald/src/read.rs"
    title: Exact seek, semantic cursor comparison, finite upper cut and endpoint revalidation
  - resource: "file:///home/rektide/src/cyclrs/source/journald/src/lib.rs"
    title: Native raw-field enumeration, source-bound cursor, budgets and finite wait
  - resource: "file:///home/rektide/src/phyllited/experiments/journal-export-receiver/src/lib.rs"
    title: Ordered repeated binary fields, exact export bytes and every-boundary fragmentation fixture
  - resource: "file:///home/rektide/src/phyllited/experiments/journal-oracle/README.md"
    title: Differential comparison preserving duplicate field pairs; privileged live-probe boundary
  - resource: "file:///home/rektide/archive/systemd/systemd/man/sd_journal_seek_head.xml"
    title: Seek-nearest behavior and required next/test exact-cursor validation
  - resource: "file:///home/rektide/archive/systemd/systemd/man/sd_journal_get_cursor.xml"
    title: Opaque cursors and multiple spellings of one native entry
verification:
  method: Independent source and research inspection; explicit-file Markdown/OKF checking
  limits: Journal implementation intentionally not read pending its owner's completion. No new runtime, Durable Streams test rerun, native journal, rotation, vacuum or power-loss evidence in this assessment.
---

# Second-stage journal integration

**Keep the two position models native. Deepen journal failure and successor
evidence before extracting a common reader or checkpoint abstraction.** This is
the independent first assessment; the parent owns Cargo-root/shared-document
integration, and Luna still owns the initial journal implementation.

## Delivered HTTP boundary reviewed

The actual [Durable Streams library](/stream/durable-streams/src/lib.rs) exposes:

| Public surface | Meaning and useful integration boundary |
| --- | --- |
| `source_url`, `Binding::new`, `Binding::validate` | Canonical HTTP(S) locator plus explicit operator-declared incarnation, media type, response bound and output profile |
| `HttpStream::read` → `Batch` | One complete bounded HTTP response, raw body and exact opaque **next unread** offset; no file state or application output |
| `FileReader::open` | Cooperative nonblocking reader lock held for the instance lifetime |
| `status`, `reset` | Bounded validated state inspection and deliberate replacement; neither contacts HTTP |
| `read_into` | Binding check, bounded read, checkpoint serialization preflight, complete write and flush, then synced temporary-file replacement and directory sync |

The read/output/save separation is substantive rather than merely naming. The
inspected tests prove refusal before output for protocol failures, source/config
mismatches, failed writes/flushes, and replay when a deterministic output writer
moves the state directory during flush. A post-rename directory-sync failure can
still mean the checkpoint is already replaced; the guide correctly disclaims a
universal old-state guarantee. No safety blocker was found in this integration
review. The workbench's eight passing tests, strict Clippy and real filesystem
server results are **attributed completed verification**, not fresh reruns here.

Two constraints matter for consumers:

- `status` is offline from the stream but opens/creates the `.lock` sidecar. It is
  not a zero-filesystem-effect forensic reader of an arbitrary immutable packet.
- A caller using `HttpStream::read` directly owns output/commit ordering. Returning
  a batch does not commit a reader, and its `up_to_date` is an observation, not a
  retention grant, stream incarnation or service authority.

URL parsing preserves URL semantics, including percent encoding; it is not path
normalization for a shared local filesystem. No authentication or ambient proxy
configuration enters this initial profile. The server does not verify the
incarnation declaration. A router cannot strengthen any of these facts merely by
assigning a resource ID.

## Journal mechanics checked independently

Current local checkout heads inspected were Cyclrs
`ab361a81b64802bc309fd20cc3f4a6098d28a20b`, Phyllited
`b6540101c1bab6466fc8fc23dc1a164b07b00421`, and archived systemd
`54b24a3855de37e5e19235c44acd113eb6ce6830`. Local source links name the actual
working files read; these HEAD coordinates alone are not a claim of clean trees
or immutable bindings for uncommitted bytes.

Cyclrs's `exact_seek` does `seek_cursor` → `next` → `test_cursor`. Its lower
resume is exclusive; its upper cut is inclusive, captured before traversal and
revalidated afterward. Crucially, it uses native semantic comparison even when
the lower cursor has a different spelling for the upper entry. This agrees with
the systemd manuals: missing cursors can seek approximately and different strings
can name one entry. Parsing or sorting a cursor is not a valid recovery method.

That is a precedent for exact positioning, **not a requirement to replace a
finite journal paginator with Cyclrs's all-or-error bounded replay policy**.
A journal batch may legitimately commit a complete emitted prefix at its own
documented entry bound. The implementation's actual policy must be reviewed and
tested on its own terms. Native handles must be opened by the successor process;
a serialized cursor is not a transferable live `sd_journal` handle.

Phyllited preserves ordered repeated fields, binary values and export wire bytes;
its test tries every split boundary. Its parser buffer is unbounded, so it is a
fidelity/test precedent rather than an unchanged dependency for bounded input.
The differential oracle retains duplicate field pairs and removes only named
storage-generated metadata. Its privileged configuration-changing script was not
run and is unnecessary for deterministic journal-reader fixtures.

## Concrete refinement acceptance cases

Once the initial journal owner reports completion, inspect its public library,
CLI, state schema and existing tests **before** selecting missing cases below.
Do not replace schemas merely to imitate the HTTP workbench.

| Case | Decisive evidence |
| --- | --- |
| Independent readers A/B | A commits a prefix, a fresh process resumes A with only the suffix; B independently starts at the declared beginning |
| Multi-identity suffix | Same unit label spans invocation/boot identities; exact cursor survives restart while all original identity fields remain visible and distinct |
| False-neighbor seek | Missing saved cursor yields an explicit gap/error, never the nearest row as successful continuation |
| Output failure | Partial write and flush failure do not commit past the declared complete output unit; retry documents possible duplicates |
| Save obstruction | Deterministic failure after successful output returns nonzero and retains inspectable state; do not assume pre-rename failure covers post-rename uncertainty |
| Binding mismatch | Changed source, namespace, selector or output interpretation is refused before source read/output; a fresh state path is an explicit alternative |
| Field fidelity | Repeated names, binary values, empty values and non-UTF-8 bytes survive the selected representation; required metadata ambiguity fails rather than silently choosing one |
| State bound/schema | Unsupported version/fields, empty or invalid position, over-bound input and malformed state fail without implicit reset |
| Installed CLI restart | Retain actual argv, stdout/stderr, exit outcome and before/after checkpoint snapshots for both attempts; compare exact output rather than only typed library results |

Reader identity, event origin, unit name, systemd InvocationID, boot ID and opaque
cursor are separate coordinates. Two launches reading the same selected history
show **reader-process succession**; they do not demonstrate that either reader
owns, restarts or controls the service whose logs it observes. A matching unit
name is not proof of one incarnation or authorization. Cross-identity fixtures
should assert preserved fields, not manufacture a global service-generation ID.

Keep fixture evidence separate from mutable checkpoints: fresh attempt directories,
raw stdout/stderr, request and outcome, before/after state, and explicit links
between prefix and successor attempts. A failed run may retain partial evidence;
missing final outcome is not success. A JSONL fixture cannot by itself qualify
native cursor equivalence, rotation, vacuum, permission loss or complete retained
history. Native read-only probes must be reported separately if exercised.

## Routing: the earned seam is description, not token translation

The [resource proposal](/design/remote-resources/draft0.gpt6a.md) already supplies
locator/kind/profile and explicit reference-base vocabulary. Use it instead of
inventing another ambient registry. HTTP(S) alone does not identify Durable
Streams; an ordinary HTTP resource can use the same scheme. A journal fixture
file and a live journal are different source kinds even when both are local.

A small independent router, **if the delivered journal interface warrants it**,
may validate/describe an explicitly selected source kind and the relevant adapter
without opening it. Its output can name the profile, supported locator form,
position semantics and required binding inputs. It must reject unsupported
kind/scheme combinations and ambiguous relative references without a base. It
must not discover services, create state, call source APIs, infer credentials or
translate one adapter's checkpoint into another.

The common conceptual record is **source selection + adapter-native position +
output boundary**. This does not yet justify a generic trait: the HTTP position
is next-unread, while native journal resume is after an exact last-entry cursor.
Both expose opaque strings, but changing a field name to `cursor` cannot erase
that difference. Leave the existing schemas intact and implement no router until
the second interface has actually been inspected.

## Handoff and verification standing

Only this addendum is owned in the first assessment. The in-progress
`stream/systemd` code/report was intentionally neither read nor edited, and no
Cargo-root, lockfile, shared index or shared guide was changed. The parent can
resume this session after the journal completion to select and implement the
missing high-value cases above. No background polling or service mutation is
needed to reach that next checkpoint.
