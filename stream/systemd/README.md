# Journal checkpoint experiment

Pre-alpha workspace crate: `handle-off-stream-journal`, binary `ho-journal`.
Finite source-bound reads with restartable file-backed positions. No async runtime,
systemd linking, host configuration changes, or activity/history policy required.

## Try the deterministic fixture

From this directory, create a fresh scratch directory and source file with an
**absolute** fixture path:

```sh
DEMO_DIR="$(mktemp -d /tmp/opencode/journal-demo.XXXXXX)"
python3 -c 'import json,pathlib; print(json.dumps({"backend":{"kind":"fixture","path":str(pathlib.Path("fixtures/journal.jsonl").resolve())},"machine_id":"a"*32,"unit":"worker.service","unit_field":"systemd_unit","selection":{"kind":"follow_unit"}}))' > "$DEMO_DIR/source.json"
cargo run --bin ho-journal -- read --source "$DEMO_DIR/source.json" --checkpoint "$DEMO_DIR/checkpoint.json" --start tail --limit 2
cargo run --bin ho-journal -- resume --checkpoint "$DEMO_DIR/checkpoint.json"
cargo run --bin ho-journal -- status --checkpoint "$DEMO_DIR/checkpoint.json"
cargo run --bin ho-journal -- reference --checkpoint "$DEMO_DIR/checkpoint.json"
```

`read` requires a new checkpoint and an explicit `--start head|tail`. `head`
starts at the oldest selected retained entry; `tail` starts from at most the latest
`--limit` selected entries. The initial position is preserved in checkpoint state.
`resume` uses the stored source; optional `--source FILE` asserts exact equality.
The checkpoint's parent directory must already exist.
Follow-unit means later invocations are eligible; this command is a finite poll,
not a background tail. `--limit` bounds emitted entries; library limits also bound
JSON line bytes and scanned bytes. Budget failure can follow already committed
entries; resume starts after the last committed output.

## Actual journal source

```json
{
  "backend": {"kind":"journal", "scope":"system", "directory":null},
  "machine_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "unit":"worker.service",
  "unit_field":"systemd_unit",
  "selection":{"kind":"follow_unit"},
  "matches":{}
}
```

Replace the machine ID with the target's `/etc/machine-id` and select an actual
unit. `directory` may select an absolute journal directory. User journals use
`scope: "user"`; user service entries use `unit_field: "user_unit"`. For a fixed runtime cycle use
`"selection":{"kind":"invocation","id":"<32 lowercase hex digits>"}`.
`matches` adds ANDed exact field matches such as `TASK_ID`, `JOB_ID`, or `_BOOT_ID`.
PID selection alone cannot identify a process lifetime. Job/task conventions are
application/manager-specific; they are not synthesized from unit or invocation IDs.

`unit_field` makes the target identity explicit:

- `systemd_unit` selects service-process entries by trusted `_SYSTEMD_UNIT`;
- `user_unit` selects user-service entries by `_SYSTEMD_USER_UNIT`;
- `manager_target` selects system manager lifecycle/job entries by `UNIT`;
- `user_manager_target` selects user-manager target entries by `UNIT`.

Manager entries have their own emitter (`_SYSTEMD_UNIT`, commonly `init.scope`),
and systemd-generated `INVOCATION_ID` (or `USER_INVOCATION_ID`) is retained as a
separate named identity rather than mistaken for service-process
`_SYSTEMD_INVOCATION_ID`. Checkpoint identity also carries `JOB_ID`, conventional
`TASK_ID`, PID and boot ID when present. Use `matches: {"JOB_ID":"..."}` to pin a
specific manager job; application-specific task names can be selected with
`matches`. A checkpoint selects one field family at a time; merging service output
and manager job output requires a caller-owned multi-source policy.

JSON output wraps the complete journalctl JSON entry with target identity,
separately named emitter/invocation identity and observation.
Repeated string fields and binary byte arrays are retained, as are nulls. This is
not byte-for-byte journal export preservation. Scalar identity interpretation is
conservative: missing/repeated identity fields produce unknown observations.
Service modes filter exactly on the selected service identity field and therefore
exclude manager messages attributed to the manager unit. `manager_target` modes
make lifecycle/job selection explicit through `UNIT`.

## Delivery contract

- A cursor is opaque. Its checkpoint `file:` URI names the state file, not an FD,
  service endpoint or cursor URI. Copying the checkpoint does not transfer ownership.
- Resume includes the saved entry, verifies its **string** cursor and skips it.
  Missing/rotated cursors fail without fallback. Equivalent alternate spellings can
  be rejected; native `sd_journal_test_cursor` is needed for semantic equality.
- Initial `head` and `tail` are explicit. Journal `tail` asks journalctl for the
  most recent bounded number of selected entries; it is not a snapshot of events
  arriving after the command starts.
- Unit-follow reads report `invocation_changed` or `boot_changed` when enough
  identity evidence exists. These observations do not prove succession, termination
  or permission to execute. PID changes within one invocation are not successors.
- Order is journal traversal order, not wallclock sort or application causal order.
  There is no frozen upper cut or atomic retained-history snapshot.
- Every emitted line is flushed before its checkpoint is atomically replaced and
  synced. A failure in that interval can replay output. A pipe flush is not downstream
  acknowledgement. Integrators needing durable sink acknowledgement should use the
  library callback and save only after their sink accepts the event.
- An advisory sidecar lock serializes cooperating CLI writers for one path; it stays
  on disk. Use one canonical checkpoint path (aliases/symlinks can bypass the lock).
  External file edits and source-path replacement are outside this contract.
- `journalctl` permissions determine visible history. Empty initial output cannot
  distinguish an empty selection from inaccessible records. No privilege escalation.
- Reads have byte and entry bounds, but no subprocess wallclock timeout. A stalled
  journalctl/filesystem can block. Early completion or error kills and reaps the child.

## Verify

```sh
cargo nextest run -p handle-off-stream-journal
cargo clippy -p handle-off-stream-journal --all-targets -- -D warnings
cargo fmt -p handle-off-stream-journal --check
```

Fixtures cover unrelated units, manager job target versus manager emitter and
distinct invocation fields, backwards wallclock, repeated/binary fields,
same-invocation PID changes, invocation change with PID reuse, reboot and missing
invocation. Synthetic missing cursor is not an induced real journal vacuum test.
See [research and evidence](/design/streams/journal0.gpt6l.md) and
[prior local readers](/design/streams/journal-prior0.gpt6l.md). Cyclrs owns the
stronger native exact-seek/frozen-cut precedent; Umbriel retains activity policy.
