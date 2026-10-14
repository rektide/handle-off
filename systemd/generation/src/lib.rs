//! V1 transient generation addressing and non-destructive discovery, not publication
//! or adoption. No manager queries, FD claiming/closing, codec, carrier or runtime.
//! Persistent listeners remain application-owned outside this generation namespace.
//!
//! Reserved grammar: `ho1.<namespace32hex>.<generation32hex>.m` (one root), `.c`
//! (commit marker), `.rXXXXXXXX` (u32 resource slot, eight lowercase hex digits).
//! Backend tokens are caller-provided 128-bit values. Generation uniqueness and
//! exclusive namespace ownership are caller policy; no RNG, truncation, hashing or
//! inode-based alias inference. Tokens are NOT arbitrary application logical IDs.
//! All generated names are 71 or 79 ASCII bytes, within systemd's 255-byte bound.
//! Unknown names are retained separately. Malformed names in the reserved `ho1.`
//! family (also bare `ho1`) fail discovery; nothing is silently merged or pruned.
//!
//! Resource slot i binds to `manifest.resources[i]`, independent of inherited FD
//! order/numbers. Declared aliases use shared ResourceId member references. A root
//! alone is never executable; marker presence is also not release authority. Later
//! adapters must validate root/marker content linkage, incarnation, compatibility,
//! custody evidence and transient retirement before any resumed I/O. This crate
//! issues no execution approvals and never selects the "latest" generation.
//!
//! Upstream service_add_fd_store checks capacity, then calls same_fd for existing
//! entries and suppresses duplicates BEFORE assigning the submitted name. A dup
//! already retained under another name therefore cannot acquire a new slot name.
//! same_fd prefers F_DUPFD_QUERY/kcmp and may fall back to inode plus open flags
//! (excluding device nodes); its proof strength depends on kernel capabilities.
//! Missing names must fail later publication verification before commit; do not
//! repair them here or infer OFD sharing from metadata equality.
#![forbid(unsafe_code)]
mod catalog;
mod name;
pub use catalog::*;
pub use name::*;
