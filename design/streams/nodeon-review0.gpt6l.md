---
type: Research
title: Nodeon stream-tooling review addendum
description: Source-checked clarifications and transfer guidance for Nodeon's implemented verifier, proposed packet reader, durable checkpoints, and source routing.
resource: /design/streams/nodeon-review0.gpt6l.md
status: draft
generated: { by: "model:openai/gpt-6-luna", at: "2026-10-22" }
sources:
  - resource: "opencode://ses_e9a2b46dcffewIMJSMjslW0hfo"
    title: Independent source review of the Nodeon stream-tooling research and reader/checkpoint/routing implications
  - resource: /design/streams/nodeon0.gpt6l.md
    title: Prior source-checked Nodeon patterns reviewed here; retained without edits
  - resource: "file:///home/rektide/src/nodeon/tool/verification/README.md"
    title: Maintained account of delivered verifier APIs, CLI, packet layout, limits and attributed checks
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/main.rs"
    title: Actual CLI routing, report readback and library-versus-CLI packet ownership
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/catalog.rs"
    title: Catalog order, selection, sequential execution, cancellation and aggregate failure retention
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/scenario.rs"
    title: Fresh packet ownership and per-file fixture materialization
  - resource: "file:///home/rektide/src/nodeon/tool/verification/build.rs"
    title: Sorted build-time scenario discovery embedded in the installed binary
  - resource: "file:///home/rektide/src/nodeon/tool/verification/src/lib.rs"
    title: Bounded reads, create-new evidence writes, report verdicts and attached identity checks
  - resource: "file:///home/rektide/src/nodeon/tool/verification/tests/catalog.rs"
    title: Selection-order and retained partial-failure regression coverage
  - resource: "file:///home/rektide/src/nodeon/tool/verification/tests/installed.rs"
    title: Installed CLI acceptance including report-only reads, real stdout bytes and cancellation
  - resource: "file:///home/rektide/src/nodeon/design/verification/README.md"
    title: Which refinements are delivered versus remaining directional proposals
  - resource: "file:///home/rektide/src/nodeon/design/verification/records0.gpt6a.md"
    title: Proposed read-only bounded packet navigation and report authority
  - resource: "file:///home/rektide/src/nodeon/design/verification/profiles0.gpt6a.md"
    title: Proposed explicit profiles distinct from tags and actual observations
verification:
  method: Read-only source, test, and design inspection at the recorded Nodeon revision
  limits: No Nodeon build, test, or verifier command was run; transfer recommendations are not protocol qualification.
---

# Nodeon stream-tooling review

This is a separate review of [the Nodeon research](nodeon0.gpt6l.md), not a
revision of that document. The inspected `~/src/nodeon` checkout remains at
`88c7de78cd71636fafa2de9a88d650f6834a752c`, the commit it records. The research's
main boundary is accurate: Nodeon has a useful installed verification driver and
retained attempts, but it is not a generic stream reader or checkpoint store.

## Clarifications for implementers

1. **Catalog order means definition order.** `Selection.ids` is a filter, not an
   ordering list: even reversed requested IDs execute in the order definitions
   entered `Catalog::new`. The existing catalog test makes this explicit. Preserve
   the distinction if a CLI later offers an explicit ordering feature.
2. **Fresh-packet ownership is not uniform across CLI verbs.** The public
   `scenario::run` and `Selected::run` APIs create and finalize their fresh packet.
   `fixture` delegates to that path; `inspect`, `invoke`, and `host-run` currently
   create the packet in the CLI and call their lower-level operation. Do not
   generalize `RunOptions`' destination contract to every command.
3. **`nodeon-verify report` is intentionally much narrower than a packet reader.**
   It reads only `report.json`, with a 16 MiB bound and schema check, then returns
   that report. It does not inventory artifacts, follow evidence links, check a
   packet hash/integrity manifest, or determine whether child records agree. The
   separate records design proposes read-only `list`/`show` navigation; that
   proposal is not an implemented dependency.
4. **Embedded discovery and caller-owned catalogs are different seams.** The CLI
   embeds a sorted build-time scan of `scenario.rs` definitions, so installed
   execution needs no checkout. Downstream Rust callers can instead construct a
   catalog from borrowed source bytes. Neither is a runtime plugin loader or a
   dynamic fixture-file CLI.
5. **The test count is attributed, not newly verified here.** The maintained guide
   reports its 18-test catalog checkpoint and example run. This review inspected
   their source/test descriptions but did not rerun them.

## Reader, checkpoint, and routing lessons

- **Start a stream evidence reader as a read-only navigation tool.** Nodeon's
  current report command is a useful CLI shape, but only a single-record read.
  The packet-reader proposal adds bounded inventory, `list`/`show`, explicit
  partial/conflict diagnostics, path containment, and on-demand bounded artifact
  reads without installed binaries or execution. For stream tools, keep this
  reader independent from source access and checkpoint mutation; a damaged or
  incomplete checkpoint should remain inspectable without silently repairing or
  restarting it.
- **Do not mistake retained-attempt writing for checkpoint persistence.** Nodeon's
  `write_json` uses `create_new` and `flush`: appropriate for immutable evidence,
  but not atomic replacement or power-loss durability. A mutable cursor needs its
  own versioned, source-bound store, single-writer policy, atomic replacement, and
  explicit durability claim. Keep emitted-output success separate from cursor
  advancement: a crash between them can replay output, and flushing stdout is not
  downstream acknowledgment.
- **Keep three routing questions explicit:** which resource/locator was selected;
  which kind/profile adapter interprets it; and whether the saved position is
  bound to that same source and selector. Nodeon's catalog ID/tag selection is
  scenario selection only; tags do not solve runtime requirements or choose an
  execution backend. Likewise, `Attached` checks an expected Nodeon generation,
  not a generic URI or stream-source identity. For stream routing, reject a
  checkpoint whose effective locator, adapter/profile, or filter differs, while
  keeping credentials out of checkpoint identity and requiring an explicit reset.
- **Use negative controls, not only a happy-path fake.** Nodeon's installed test
  copies tools out of the checkout, checks report-only readback does not rerun a
  workload, asserts exact child stdout, rejects stale generations before request
  creation, and proves missing custody cannot pass. For a reader/checkpoint CLI,
  add equivalent tests for wrong source/filter, malformed state, output/flush
  failure, save failure, and restart replay. A scripted adapter with explicit
  read/output/save barriers is stronger than timing sleeps for crash-window cases.

## Source map and standing

The pinned links below identify the source reviewed; the maintained guide is the
best entry point for commands and current behavior.

| Evidence | What it establishes |
| --- | --- |
| [Maintained driver guide](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/README.md) | Implemented command/API boundary, packet layout, limits, and attributed verification |
| [`main.rs`](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/src/main.rs) | `report` reads one bounded report; fixture delegates to the catalog runner; other verbs retain CLI-owned packets |
| [`catalog.rs`](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/src/catalog.rs) and [catalog tests](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/tests/catalog.rs) | Definition-order selection, sequential execution, unstarted cancellation IDs, and retained child-finalization failures |
| [`scenario.rs`](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/src/scenario.rs) and [`build.rs`](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/build.rs) | One-file-at-a-time materialization and sorted build-time discovery |
| [Installed acceptance](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/tool/verification/tests/installed.rs) | Actual installed CLI behavior, exact bytes, no-rerun report read, stale-generation refusal, and signal cleanup |
| [Refinement index](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/design/verification/README.md) and [packet-reader design](https://github.com/rektide/clid/blob/88c7de78cd71636fafa2de9a88d650f6834a752c/design/verification/records0.gpt6a.md) | Catalog refinement #1 is delivered; reader, record links, fixture assembly, and profiles remain proposals |

For handle-off, borrow the useful boundaries, not Nodeon's implementation types:
the owner/client/attached-service types are product-specific. Keep source
resolution, stream positioning, output commitment, and offline evidence reading
separately testable. The [original research](nodeon0.gpt6l.md) remains the broader
account of Nodeon's transferable patterns and its explicit non-reuse limits.
