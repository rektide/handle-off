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

The foundation provides independent resource, bundle, opaque state, invocation,
and stdio crates, plus raw Unix datagram descriptor transfer. Bundle delivery
protocols, systemd integration, and cooperative lifecycle coordination are not
implemented yet. The delivery sequence is proposed in
[design/fd-handoff/architecture0.gpt6a.md](design/fd-handoff/architecture0.gpt6a.md),
with the reading map in
[design/fd-handoff/README.md](design/fd-handoff/README.md).

## Workspace

| Crate | Description |
| --- | --- |
| `handle-off` | Facade with empty defaults and opt-in reexports |
| `handle-off-handles` (`handles/core`) | Logical resource IDs and explicit owned/borrowed FD tables |
| `handle-off-bundle` (`handles/bundle`) | Multiple named bundles and complete manifest membership checks |
| `handle-off-state` (`state/core`) | Opaque versioned extensions with inline or resource-range payloads |
| `handle-off-stdio` (`handles/stdio`) | Explicit borrowed/owned stdin, stdout, stderr roles |
| `handle-off-invocation` (`invocation/core`) | Native lossless argv, environment, and cwd snapshots |
| `handle-off-unix-rights` (`handles/unix-rights`) | Blocking Unix datagram payload/FD transfer using rustix |

Foundation/local crates have **no external dependencies**. Facade features `handles`, `bundle`,
`state`, `stdio`, and `invocation` enable individual reexports; `local` enables
only `stdio` and `invocation`. Descriptor APIs require Unix. Native invocation
types and metadata can be used without descriptor operations.
The separate `unix-rights` feature adds rustix; it is not included in `local`.

## Local use

```toml
[dependencies]
handle-off = { version = "0.1.0", features = ["local"], default-features = false }
```

This is the intended dependency declaration; packages have not been published.
Use workspace path dependencies for now. The example handler receives invocation
and stdio explicitly, copies stdin bytes to stdout, and writes a summary to stderr:

```sh
cargo run -p handle-off-local-example -- arbitrary arguments < input > output
cargo nextest run --workspace --all-features
```

Arguments include argv[0]; Unix non-UTF-8 arguments, environment, and cwd remain
native bytes. Invocation capture does not execute a command or change process
globals. Do not mix buffered standard-I/O access with direct descriptor I/O
unless the application coordinates buffering.

Resource IDs express caller-declared associations, not FD-number/OFD identity.
Manifest validation checks structure and complete membership; applications still
validate extension compatibility, resource kinds, carrier bounds, and the right
to resume I/O. Descriptor duplication preserves shared offsets/status flags and
does not revoke the original owner's access.

See the [foundation implementation report](doc/foundation/implementation0.gpt61s.md)
for APIs, extraction ownership, checks, and the measured local-build baseline.

## Raw descriptor transfer

`handle-off-unix-rights` sends a nonempty payload plus borrowed FDs as one Unix
datagram and receives owned FDs with explicit size/count limits. Sender ownership
is retained. Receive rejects payload/control truncation and closes acquired
references before returning an error. No stream framing or application adoption
permission is implied; successful send is not an acknowledgment.

The independent two-process example transfers a file reference. Use a private
directory and run receive/send in separate terminals:

```sh
umask 077
mkdir -p .test-agent/rights-demo
cargo run -p handle-off-raw-rights-example -- receive .test-agent/rights-demo/socket
cargo run -p handle-off-raw-rights-example -- send .test-agent/rights-demo/socket README.md
```

The receiver removes its socket path after receipt; it never removes a preexisting
path before binding. Failures can leave the path for deliberate cleanup. The raw
primitive does not authenticate peers; applications own endpoint permissions and
adoption policy. Received FDs have CLOEXEC; atomicity of that setting varies by
Unix platform. See the [rights checkpoint](doc/transport/rights0.glm53max.md).
