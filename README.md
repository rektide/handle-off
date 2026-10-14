# handle-off

> Pass, retain, and recover access to open resources, together with the
> information needed to continue using them.

`handle-off` is a Rust toolkit for resource handoff: direct descriptor
passing, systemd fdstore custody, socket activation, and cooperative
state-carrying handover. Stdio is one bundle; established network connections
are another. Linux is the initial systemd/memfd integration target; portable
Unix mechanisms remain separately buildable, and no async runtime is
mandatory.

This workspace is a fresh start. The retired `clid` experiment (a CLI→daemon
execution hand-off prototype, itself renamed from `prefork`) is not a
compatibility constraint; its code lives in version history, with its code
review in [REVIEW-0.md](REVIEW-0.md) and prior planning in `.transcript/`.

## Status

The workspace is a minimal foundation scaffold. Crate boundaries, APIs, and
the delivery sequence are proposed in
[design/fd-handoff/architecture0.gpt6a.md](design/fd-handoff/architecture0.gpt6a.md),
with the reading map in
[design/fd-handoff/README.md](design/fd-handoff/README.md).

## Workspace

| Crate | Description |
| --- | --- |
| `handle-off` | Facade anchor; capability crates will reexport here via opt-in feature groups |
