---
type: ImplementationReport
title: Journal stream references and conservative resume
description: Experimental journalctl-backed event stream, source-bound checkpoints and observable invocation changes.
resource: /design/streams/journal0.gpt6l.md
status: draft
generated: { by: "model:openai/gpt-6-luna#xhigh", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a2e6661ffepeqA79um9bzmf5"
    title: Initial systemd research under Astra medium; after explicit switch, Luna xhigh completed CLI, fixture behavior, tests and journalctl probe
  - resource: /design/streams/journal-prior0.gpt6l.md
    title: Recovered Cyclrs exact-cursor mechanics, Phyllited raw-field fixtures and Umbriel ownership boundary
  - resource: /design/streams/nodeon0.gpt6l.md
    title: Source-binding, checkpoint-write, explicit initial-tail and journal cursor design recommendations
  - resource: /design/remote-resources/draft0.gpt6a.md
    title: Resource descriptions, explicit resolution context and local realizations
  - resource: /doc/systemd/inspection0.gpt61s.md
    title: Observation versus authority and service incarnation bracketing
  - resource: /doc/systemd/generations0.gpt61s.md
    title: Backend addresses versus logical generations
  - resource: file:///home/rektide/archive/systemd/systemd/man/sd_journal_get_cursor.xml
    title: Opaque cursors and non-unique cursor strings
  - resource: file:///home/rektide/archive/systemd/systemd/man/sd_journal_seek_head.xml
    title: Seek followed by movement and exact-position testing
  - resource: file:///home/rektide/archive/systemd/systemd/man/systemd.journal-fields.xml
    title: Machine, boot, process and invocation identities
  - resource: file:///home/rektide/archive/systemd/systemd/src/journal/journalctl-show.c
    title: journalctl after-cursor fallback behavior
  - resource: file:///home/rektide/archive/systemd/systemd/man/journalctl.xml
    title: Cursor, cursor-file, line-count and no-tail command semantics
---

# Journal references and resume

Pre-alpha experiment. The initial prototype/research was authored under Astra
medium before the explicit model switch. Luna xhigh completed the implementation,
fixture and live-command verification in this same session; provenance preserves
that division rather than attributing the earlier phase to Luna.

## Evidence and backend choice

Archived systemd revision `54b24a3855de37e5e19235c44acd113eb6ce6830`:

- `man/sd_journal_get_cursor.xml:46–73`: cursors are opaque; multiple strings
  may name the same entry. Native equality uses `sd_journal_test_cursor`.
- `man/sd_journal_seek_head.xml:81–104`: seek alone does not select an entry;
  movement follows, and unavailable cursors land on a nearest entry.
- `src/journal/journalctl-show.c:73–98`: `--after-cursor` skips only an exact
  match, otherwise continues from the fallback. It does not enforce no-gap resume.
- `man/journalctl.xml:253–269`: `--cursor` is inclusive; `--after-cursor` is
  available, but the adapter uses inclusive cursor plus its own first-entry check.
- `man/journalctl.xml:684–690, 787–790`: line selection bounds output; `--no-tail`
  explicitly undoes line limits.
- `man/systemd.journal-fields.xml:292–316`: machine, boot, and unit runtime-cycle
  invocation IDs supply distinct identity scopes.
- `man/systemd.journal-fields.xml:104–111`: manager `INVOCATION_ID` is distinct
  from trusted `_SYSTEMD_INVOCATION_ID`, even though each can identify a unit cycle.

Use installed `journalctl` JSON output for an accessible, runtime-free first
adapter. Request the saved cursor inclusively, require its emitted cursor string
to equal our saved string, then skip it. `--lines=all` is set for replay; initial
`head` is unbounded by journalctl, while `tail` uses the requested batch count.
Initial mode is required by the CLI and retained in state. This is deliberately **conservative**:
an equivalent alternate cursor spelling can be rejected. It never parses cursor
internals or assumes timestamps are positions. A native adapter with
`sd_journal_test_cursor` is the next improvement for semantic equality.

The first non-bounded exploratory `journalctl` probe included `--no-tail`; systemd
documents that this undoes `--lines` and emits all stored lines, so the adapter
does not use it. The actual unit probe used a bounded tail after that correction.

## Identity and state

The checkpoint contains an explicit source selection, last emitted opaque cursor,
and last observed identity. Machine and target unit are required. `unit_field`
selects trusted `_SYSTEMD_UNIT`, `_SYSTEMD_USER_UNIT`, manager `UNIT`, or user-manager
`UNIT`. Manager targets and service-process sources are separate source families;
there is no implicit union or hidden broadened `--unit` match. Fixed service
invocations select `_SYSTEMD_INVOCATION_ID`; manager targets select the correspondingly
named system/user manager `INVOCATION_ID`. The identity record names which invocation
field supplied the value and preserves the manager emitter separately from the
target unit. Job and conventional task IDs remain separate identity coordinates.
Boot and invocation changes are observations, not proof an old worker stopped or
execution permission transferred. Missing identity produces unknown rather than
inference. PIDs may be reused, and a unit may contain several processes.

The checkpoint's `file:` URI references a local state document, not a journal
cursor, descriptor or globally resolvable service. Source settings remain attached
to that document. Paths must be absolute; directory/fixture replacement at the same
path remains an external administrative possibility. No authentication or lineage
proof is claimed by the URI or machine ID.

## Delivery and limits

Reads are finite batches in journal traversal order. Follow-unit describes identity
selection, not a continuously running tail process. Repeated resume polls make
progress. Retention or filtering changes that remove the saved entry fail closed;
starting over requires a new checkpoint chosen by the caller. There is no automatic
reset or prune. Journal ordering is not application causal order; wallclock time
can go backwards and monotonic timestamps are boot-scoped.

Initial `head`/`tail` applies only before the first cursor is committed. Fixture tail
scans within the configured byte limits and buffers at most the last batch of
matching events; the journal backend asks journalctl for that bounded tail. Neither
is a live subscription or an atomic snapshot of concurrent appends.

## Verification observed

After removing `--no-tail`, an unprivileged local invocation against an actively
emitting system service completed `read --start tail --limit 1`, then
`resume --limit 1`. Both returned an entry with an opaque `__CURSOR`; the resumed
entry had the same machine, boot, target unit, invocation and PID as the prior
entry, with observations `first` then `same_invocation`. This demonstrates the
installed journalctl argument/output path on this host only. No journal
configuration was changed; no rotation, access-loss or service-restart condition
was induced.

The standalone crate passed **9 nextest integration tests**, `cargo clippy
--all-targets -- -D warnings`, and `cargo fmt --check`. The tests include fixture
head/tail/read/resume/status/reference CLI flow, source-mismatch no-write, atomic
state roundtrip/no-clobber, writer exclusion, consumer failure, byte budgets,
missing cursor, binary/repeated values, invocation/boot changes, and manager target
versus emitter identities. Rekon `check` and `check --okf` passed for the report
and ordinary experiment README. These results do not test a real journal retention
gap or establish exactly-once sink delivery.

The CLI holds an advisory exclusive sidecar lock across read/output/commit.
Checkpoint replacement uses same-directory temporary write, file sync, rename and
directory sync. Cooperating writers serialize; external editors need not cooperate.
Output is flushed before saving each entry. A crash or storage failure between output
and checkpoint creates replay; a successful stdout flush does not prove downstream
durability. This is not exactly-once processing or a transactional sink.

## Connections

- [Resource direction](/design/remote-resources/draft0.gpt6a.md): a locator describes
  what to resolve; a checkpoint is a local realization of consumer position.
- [Inspection](/doc/systemd/inspection0.gpt61s.md): identity observations never grant
  execution permission. Journal successor observations have weaker temporal evidence.
- [Generation catalogs](/doc/systemd/generations0.gpt61s.md): generation addresses
  remain a distinct backend vocabulary; journal cursors are not FD-store names.
- [Experiment usage](/stream/systemd/README.md): commands, fixtures and limitations.
