---
type: Research
title: Prior journald reading research for stream fixtures
description: Cotail recovery of today's journald survey, key reader and fixture sources, and the limits of the session search.
resource: /design/streams/journal-prior0.gpt6l.md
status: draft
generated:
  at: 2026-10-22
  by: model:openai/gpt-6-luna#high
sources:
  - resource: opencode://ses_e9a2f12d5ffeI73SSmhA3XtDNk
    title: Recovered earlier journald survey through read-only cotail search and linked its source paths
  - resource: opencode://ses_e9a70537bffeI1H23pKsQ37jBa
    title: Earlier subagent survey of recent local journald readers and activity-tracking connections
  - resource: file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md
    title: Maintained survey with dated evidence and detailed reuse limits
verification:
  method: Cotail CLI help inspection and read-only content search; read cited survey and local implementation/design documents
  limits: Cotail search excerpts did not expose matched message IDs; references below are exact session-level URIs. No source code or runtime behavior was reverified here.
---

# Prior journald reading research

Today's earlier research is recoverable; the maintained handoff is
[ZK's journald survey](file:///home/rektide/src/zooming-kittens/design/activity/journald-survey0.gpt6a.md).
It is a dated June 22–October 22, 2026 survey, with an explicit warning that
its Git dates identify source history rather than deployment and that its
inventory is not proof of completeness.

## Prior sessions

- [Research journald activity precedents — `ses_e9a70537bffeI1H23pKsQ37jBa`](opencode://ses_e9a70537bffeI1H23pKsQ37jBa), a subagent session on October 22 in `~/src/zooming-kittens`. Cotail surfaced its conclusion: Cyclrs is the strongest reusable native reader; Phyllited is useful for binary-safe parsing and differential fixtures; ZK's `journal.sh` is only a human-facing follower.
- [Research Umbriel longcat integration — `ses_e9a70297dffe6dQ5UJP0up8m7T`](opencode://ses_e9a70297dffe6dQ5UJP0up8m7T), later the same day, records that the Cyclrs/Phyllited comparison was committed as `design/activity/journald-survey0.gpt6a.md` in that checkout. Cotail's cursor-focused excerpt says exact-cursor bounded replay is not a complete live-follow loop.
- The current survey's own source provenance points back to the first session and lists the exact source files and its evidence limits. Cotail's search output did not provide message IDs, so these are exact native session references, not message-specific links.

## What to carry into the fixture work

1. **Cyclrs** (`~/src/cyclrs/source/journald/`): inspect `README.md`, `src/lib.rs`,
   and `src/read.rs`. The survey describes exact cursor validation
   (`seek_cursor` → `next` → `test_cursor`), bounded replay with inclusive upper
   cut and exclusive lower resume, raw-field enumeration, and entry/byte/time
   budgets that fail rather than silently truncate. Its provider-specific types
   and codec make it a mechanics precedent, not a drop-in Umbriel dependency.
2. **Phyllited** (`~/src/phyllited/experiments/`):
   `journal-export-receiver/src/lib.rs` preserves ordered repeated fields,
   binary values and wire bytes; its fixture is split at every byte boundary.
   `journal-oracle/README.md` compares forwarded entries to `journalctl -o
   export` while retaining duplicate field pairs. This is a fixture/oracle
   precedent, not retained-history cursor compatibility. Its live script changes
   journal configuration with privilege and was not run in the survey.
3. **Umbriel** (`~/src/umbriel-rs/design/activity/`):
   `records0.gpt6a.md` selects field-first tracing and accepted identities plus
   metadata updates; `service0.gpt6a.md` owns the service/history commission;
   `prior-art0.glm53fx.md` is the earlier survey. The open
   `umbrs-activity-history` work item already owns cursor-aware follow and links
   Cyclrs. Continue there rather than opening duplicate history ownership.
4. Keep replay semantics separate from the live-follow design: historical/live
   handoff, cancellation, backpressure, checkpoint timing, and initial-tail
   policy still need explicit treatment. A surviving old snapshot or successful
   write/readback does not prove no suffix was lost.

The survey additionally points to `phyllited/.design/journald/otel0.gpt56.md`
for a lossless OTel mapping proposal, and warns that boot, invocation, cursor,
and synthetic stream IDs are separate identity coordinates. Its recommended
fixtures are unprivileged and deterministic by default; do not claim they test
rotation, vacuum, or access loss unless those conditions are actually induced.

## Search scope and tooling

The installed checkout is `/home/rektide/src/opencoattails`; its CLI is run as
`node src/cli.ts` (the `cotail` executable was not on this shell's `PATH`). I
inspected root, `search`, and `history` help before searching. The targeted
read-only searches used `cotail search` with `--since-updated 1d`, disabled
backfill where applicable, and then narrowed results to
`/home/rektide/src/zooming-kittens` with `--subagent` and terms `journal`,
`cursor|position|seek`, and `boot|InvocationID|restart`. The research session
was found among the matching results.

`history --since 1d --json --subagent --limit 100` also confirmed today's
matching ZK work and its session ID. This was not an exhaustive archive query:
it used the configured default source profile and a one-day update window for
initial discovery, then the ZK directory filter for focused content search.
Older or differently indexed sessions may therefore be missed. No session or
Cotail configuration was changed.
