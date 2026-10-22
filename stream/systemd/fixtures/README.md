---
type: ImplementationGuide
title: Retained journal reader fixtures
description: Run the installed CLI outside the checkout and retain explicit reader-succession, independent-reader and refusal evidence.
resource: /stream/systemd/fixtures/README.md
status: experimental
generated: { by: "model:openai/gpt-6.1-sol#xhigh", at: 2026-10-22 }
sources:
  - resource: "opencode://ses_e9a25db9cffeW8IeTiVrCNzdoC"
    title: Sol xhigh implements and runs the retained installed-reader fixture and documents its evidence and limits
  - resource: /stream/systemd/fixtures/installed.py
    title: Actual fresh-run, command capture, state snapshots and explicit evidence relationships
  - resource: /design/streams/integration0.gpt61s.md
    title: Source review and concrete installed/live probe observations
---

# Journal reader fixtures

`journal.jsonl` is synthetic journalctl JSON. It deliberately includes unrelated
entries, manager target/emitter identities, repeated strings, binary bytes, nulls,
backwards realtime, invocation changes, PID changes/reuse and a boot transition.
It does not encode native journal files or qualify cursor aliases/retention.

From the repository root, run the built binary outside the checkout's working
directory and retain a fresh evidence packet:

```sh
cargo build -p handle-off-stream-journal --bin ho-journal
python3 stream/systemd/fixtures/installed.py \
  --client target/debug/ho-journal --scratch .test-agent/journal-integration
```

Each run makes a new `installed-*` directory, copies the explicitly selected client
and fixture, and records their SHA-256, actual argv/PIDs, raw stdout/stderr,
exit outcomes, and checkpoint bytes before/after each attempt. The top-level
`evidence.json` links reader A's prefix to its new-process successor and to
independent reader B. It also records caught-up resume, reusable checkpoint
`file:` reference, offline source description, wrong-source and missing-cursor
refusal. Failed assertions retain the packet with failed status; missing attempt
outcome is incomplete evidence, not success. Paths and hashes are local evidence,
not authentication or a signed integrity manifest.

The script launches only its copied reader binary, with null stdin and file-backed
output; each call has a finite timeout and kills/reaps its direct child on failure.
It never changes journal configuration, restarts services, removes older packets
or resolves a resource through a service registry. This is a trusted local fixture,
not hostile-process containment. Five selected events are expected from this
specific fixture; `--fixture` chooses the input file explicitly, not arbitrary
scenario semantics.

Rust tests exercise the same installed-process/cross-identity behavior and add
deterministic partial-output, flush, save-obstruction, state bounds and a scripted
nearest-neighbor `journalctl` negative control. The scripted command does not
constitute native vacuum/rotation testing. The
[integration addendum](/design/streams/integration0.gpt61s.md) distinguishes these
results from live journal evidence and explains the conservative cursor policy.
