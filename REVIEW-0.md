# clid Code Review

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     clid-client (Client)                         │
│  ExecutionContext → rkyv serialize → sendfd Transfer            │
└──────────────────────────┬──────────────────────────────────────┘
                           │ Unix Datagram + FDs
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     clid-daemon (Daemon)                         │
│  Unix Socket Receiver → rkyv Deserialize → CmdRegistry dispatch │
└─────────────────────────────────────────────────────────────────┘
                           │
              ┌────────────┴────────────┬──────────────┐
              ▼                         ▼              ▼
     clid-discovery            clid-context      clid-client
  (socket discovery)      (ExecutionContext)   (send helpers)
```

---

## Critical Issues

### 1. Wildcard Dependency Versions

**File:** `Cargo.toml` (lines 12-21)

```toml
clap = { version = "*", ... }
anyhow = "*"
thiserror = "*"
tokio = { version = "*", ... }
```

Using `*` breaks reproducibility and can pull semver-incompatible changes.

**Fix:** Pin specific versions in workspace dependencies.

### 2. Daemon Receive Loop Anti-Pattern

**File:** `clid-daemon/src/lib.rs`

Current implementation uses:
- `spawn_blocking` per packet
- `Arc<Mutex<std::net::UnixDatagram>>`
- Manual `sleep(10ms)` on `WouldBlock`

This creates unnecessary tasks, lock contention, and CPU spin.

**Fix:** Replace with `tokio::io::unix::AsyncFd` around the std socket and call `recv_with_fd` inside `try_io`.

---

## Security Concerns

### High Priority

1. **Socket Placement + Permissions**
   - Binding in `./` is risky
   - `std::fs::remove_file(&socket_path).ok()` before bind can delete arbitrary files
   - **Fix:** Use `$XDG_RUNTIME_DIR` with directory permissions `0700`

2. **Unauthenticated Local IPC**
   - Any process that can reach the socket can submit commands
   - No peer credential checking
   - **Mitigation:** Enforce socket only accessible to same UID via filesystem permissions

3. **rkyv and Untrusted Bytes**
   - Must ensure validated deserialization APIs are used
   - Currently using `from_bytes::<ExecutionContext, Error>` which should validate with bytecheck

### Medium Priority

4. **DoS Vectors**
   - No rate limiting / concurrency limits
   - No payload size limit (buffer is fixed 16384, truncation causes corrupted data)
   - No per-command timeout

---

## Code Quality Issues

### FD Handling

**File:** `clid-daemon/src/lib.rs`

Repeated `unsafe` conversions: `OwnedFd → raw → File::from_raw_fd`.

**Fix:** Prefer safe conversions: `std::fs::File::from(OwnedFd)` then `tokio::fs::File::from_std`.

### Client Blocking in Async

**File:** `clid-client/src/lib.rs`

`send_with_fd` is synchronous but called in async context. Can block runtime thread in pathological conditions.

---

## Testing Coverage Gaps

### Meaningless Tests

**File:** `clid-discovery/src/xdg_runtime.rs`

```rust
assert!(result.is_none() || result.is_some());  // Always true
assert_eq!(strategy.discover(), strategy.discover());  // Tautology
```

### Missing Integration Tests

- Real FD transfer (stdin/stdout/stderr)
- End-to-end client→daemon dispatch
- Socket discovery precedence (CLI vs env vs filesystem)
- Payload size rejection / invalid data rejection

### Test Race Conditions

Environment variable tests use `unsafe set_var/remove_var` which can race in parallel test execution.

---

## Dependency Management

### Inconsistent Workspace Usage

`clid-context` pins its own dependency versions instead of using workspace deps.

**Fix:** Use `dep.workspace = true` consistently.

---

## Recommended Fixes by Effort

### Small

- [ ] Pin dependency versions (remove `*`)
- [ ] Fix meaningless tests
- [ ] Normalize workspace dependency usage in clid-context

### Medium

- [ ] Replace daemon receive loop with `AsyncFd`
- [ ] Safe socket cleanup (verify file is socket before removal)
- [ ] Add payload size checks + protocol header (magic bytes, version, length)

### Large

- [ ] End-to-end integration tests with real FD transfer
- [ ] Authentication / peer credential checking
- [ ] Background launch / pidfiles / sd-notify
- [ ] Stream socket protocol (if larger payloads needed)

---

## Risks and Guardrails

1. **If daemon ever runs with higher privileges**, current model is privilege-escalation risk
   - Keep same-UID only via private runtime dir + strict perms
   - Reject requests unless peer UID matches
   - Do not implement "execute arbitrary command" without allowlist

2. **Serialization safety**: treat bytes as hostile; ensure rkyv validation and protocol versioning

3. **Datagram truncation**: enforce length checks; reject oversize payloads
