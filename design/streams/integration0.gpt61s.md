---
type: ImplementationReport
title: Second-stage journal integration and stream routing assessment
description: Independent reader review, journal output/checkpoint hardening, explicit source description and reusable references, with installed-process and read-only live evidence.
resource: /design/streams/integration0.gpt61s.md
status: draft
generated: { by: "model:openai/gpt-6.1-sol#xhigh", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a25db9cffeW8IeTiVrCNzdoC"
    title: Explicitly selected Sol xhigh independently reviews both delivered readers, implements bounded control documents and tested journal commitment/routing/reference refinements, and retains installed-process and live read-only evidence
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
  - resource: /design/streams/journal0.gpt6l.md
    title: Released journal implementation account and attributed initial checks
  - resource: /stream/systemd/src/lib.rs
    title: Reviewed native journal schema and implemented bounded loaders, output commitment, source description and reference parsing
  - resource: /stream/systemd/tests/output.rs
    title: Deterministic partial-write, flush, save-obstruction, state-bound and ambiguous-identity checks
  - resource: /stream/systemd/tests/installed.rs
    title: Copied-binary process succession, independent readers, source binding, references, offline description and nearest-neighbor negative control
  - resource: /stream/systemd/fixtures/installed.py
    title: Maintained installed-reader fixture with retained raw output, state snapshots, input hashes and explicit attempt relationships
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
  method: Independent source review, scoped nextest/Clippy/format checks, retained installed-binary fixture, read-only live journal probe and explicit-file Markdown/OKF checking
  limits: Exact-string journalctl cursor equality only, not native semantic equivalence. No Durable Streams test rerun, induced native retention gap, service takeover, rotation, vacuum, permission loss or power-loss qualification.
extensions:
  related_work: [handoff-streams-source-routing]
---

# Second-stage journal integration

**Keep the two position models native. Deepen journal failure and successor
evidence before extracting a common reader or checkpoint abstraction.** The first
assessment was committed as `f8feadca` without reading Luna's unfinished journal
work. After Luna released `fbc31b11`, the parent resumed this session for the
implemented refinement below. Cargo-root integration and shared documentation
remain parent-owned. This work belongs to `handoff-streams-source-routing`.

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

The first assessment proposed these acceptance cases. After inspecting the released
library/CLI/state/tests, the implementation below selected missing cases without
replacing either native checkpoint schema.

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
that difference. The delivered interface justified a small **journal-specific
description command**, not a separate `stream/router` package or universal reader.

## Implemented journal refinement

The released journal reader already correctly separates manager target identity
from emitter unit/PID, uses explicitly chosen field families, preserves journalctl
JSON arrays/nulls, and stops rather than accepting an approximately sought cursor.
It commits complete entries one at a time, not an all-or-error replay. No native
libsystemd backend was added: conservative alias rejection remains useful and
honestly documented, while native equality/retained-range semantics warrant their
own fixture and dependency decision rather than an incidental replacement.

Two real gaps were fixed: `Checkpoint::load` and CLI source loading had no input
bound, and the CLI's write/flush/save boundary had no directly testable public seam.

- `Source::load` and `Checkpoint::load` now bounded-read 64 KiB control documents;
  oversized, malformed, unsupported-version and unknown-field state fails closed.
  Saved checkpoints use the same bound, preflighted before output. The schemas
  remain unchanged. Extreme line limits fail as ordinary errors, avoiding overflow.
- `Checkpoint::emit_into` builds the prospective checkpoint/observation, validates
  serialization, writes a complete JSONL line, flushes, then performs the synced
  replacement. It updates in-memory state only on success. CLI and tests use this
  exact method. Callers still own `CheckpointLock`, ordering and source traversal;
  application-acknowledged sinks can still use the existing read callback instead.
- Partial write, flush and deterministic post-output/pre-rename save failure leave
  the old disk position and produce documented replay. The save fixture moves the
  state directory during output flush; reopening at its retained location emits
  the **exact same** observation line. This is not evidence for post-rename sync
  failure or machine power loss. Such a failure may already have replaced state;
  stop and inspect it rather than assuming the old disk position.
- `Source::describe` and `ho-journal describe --source FILE` report explicit source,
  effective selectors, backend profile, output profile and position/comparison
  semantics without source or checkpoint access. Missing target directories work
  for description. No registry, credential lookup, service discovery or checkpoint
  conversion is implied; source paths remain explicitly absolute.
- All CLI `--checkpoint` arguments now accept the local absolute `file://` URI
  printed by `reference`, or native paths including non-UTF-8 bytes. Remote hosts,
  queries/fragments and ambiguous `file:relative` inputs fail. Percent-encoded `#`,
  spaces and non-UTF-8 bytes round-trip. URI parsing does not inspect a cursor or
  identify a service; it resolves a local mutable checkpoint document.

The tests copy the built binary to a temporary directory outside the checkout's
working directory. Reader A emits two entries, exits, then a fresh process resumes
from its printed URI through invocation and boot changes. Reader B independently
reads all five selected entries. A's concatenated observation lines equal B's,
and every complete event value equals the fixture's selected entry. PID changes
within one invocation and PID reuse across invocations remain separate from reader
PIDs. Binary/repeated/null/empty values survive; repeated invocation metadata stays
uninterpreted/unknown, while a non-scalar cursor fails before output.

Six source changes are rejected before source access even after the original
fixture is removed: unit, machine, boot filter, fixed invocation, field family and
backend path. Missing/alternate fixture cursor strings leave state unchanged.
A scripted `journalctl` returns a false nearest row, confirming inclusive
`--cursor`, `--lines=all` and refusal before output. This negative control is not
native rotation/vacuum evidence and does not claim to recognize native aliases.

## Actual verification and retained evidence

From the repository root:

```sh
cargo nextest run -p handle-off-stream-journal
cargo clippy -p handle-off-stream-journal --all-targets -- -D warnings
cargo fmt -p handle-off-stream-journal --check
cargo build -p handle-off-stream-journal --bin ho-journal
python3 stream/systemd/fixtures/installed.py \
  --client target/debug/ho-journal --scratch .test-agent/journal-integration
python3 .test-agent/journal-integration/live.py
```

The maintained installed fixture passed at
`.test-agent/journal-integration/installed-6kuocgfd/evidence.json`. It retains the
copied client's SHA-256, exact fixture/source inputs, actual argv/PIDs, raw
stdout/stderr, exit outcomes, before/after checkpoint bytes and explicit links
between prefix/successor/independent attempts. Wrong-source/missing-cursor
refusals and offline description are also recorded. Fresh run directories are
never reused or automatically deleted. See the
[fixture guide](/stream/systemd/fixtures/README.md) for invocation and limits.

A separate **read-only live probe** passed at
`.test-agent/journal-integration/live-e5_evqfr/evidence.json`. Scratch `live.py`
retains bounded `journalctl` discovery and version output, selects an actually
observed machine/system-unit pair, then runs describe, tail-one read, reference,
resume through that reference and status. It selected
`k3s-server-workhorse-voodoowarez-com.service`, emitted one entry and resumed with
zero new entries. The saved opaque cursor matched the raw emitted row exactly;
the encoded `#` checkpoint reference was reusable. **This proves a caught-up
resume on this host, not a new live suffix or service succession.** Cross-identity
suffix evidence comes from the deterministic fixture. Neither journal configuration
nor services were changed. Scratch paths are local retained observations, not
portable evidence links or committed journal payloads.

Final scoped verification passed: **19 nextest tests**, strict Clippy with
`--all-targets -- -D warnings`, and package-scoped format checking. The original
nine tests are retained; ten new tests cover the refinement. Rekon checks passed
for this addendum and both guides, including `--okf` for structure, destinations,
anchors and actual session provenance. The initial nine-test report remains
attributed to Luna rather than overwritten.

Only `stream/systemd` implementation/tests/fixtures/guides and this addendum were
changed in this refinement. Root Cargo files, shared guides, other research and
tickets remain with the parent. No additional agent or native journal library
dependency was introduced.

Recorded checkpoints: independent assessment `f8feadca`; completed implementation
and fixture/guide stage `212d90b1` (19-test/Clippy/format/document checks and both
retained probes passed before that commit). This report is preserved separately
so the implementation's concrete verification can be followed from the parent
integration and ticket without rewriting Luna's original account.
