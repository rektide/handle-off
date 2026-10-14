//! Single-threaded synthetic socket-unit launcher for isolated tests.
//! It changes only its OWN descriptor table, then execs the activation consumer.
#[cfg(unix)]
fn main() -> std::io::Result<()> {
    use std::{
        net::TcpListener,
        os::{
            fd::{AsRawFd, FromRawFd, OwnedFd},
            unix::process::CommandExt,
        },
        process::Command,
    };
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err(std::io::Error::other("usage: MANAGER CONSUMER MODE"));
    }
    let mode = args[2]
        .to_str()
        .ok_or_else(|| std::io::Error::other("invalid mode"))?;
    let metrics = TcpListener::bind("127.0.0.1:0")?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let copies = [
        rustix::io::fcntl_dupfd_cloexec(&metrics, 10)?,
        rustix::io::fcntl_dupfd_cloexec(&listener, 10)?,
    ];
    drop(metrics);
    drop(listener);
    let mut inherited = Vec::new();
    for (index, source) in copies.iter().enumerate() {
        let target = 3 + index as i32;
        // SAFETY: ordinary dup2 syscall into this single-threaded fixture process.
        // Sources are live high-numbered owners; target slots are deliberately
        // reserved for activation. No source can be overwritten by the targets.
        if unsafe { libc::dup2(source.as_raw_fd(), target) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: dup2 created a valid descriptor for which this fixture is sole owner.
        inherited.push(unsafe { OwnedFd::from_raw_fd(target) });
    }
    let mut command = Command::new(&args[1]);
    command.env_remove("LISTEN_PIDFDID");
    match mode {
        "valid" => {
            command
                .env("LISTEN_PID", std::process::id().to_string())
                .env("LISTEN_FDS", "2")
                .env("LISTEN_FDNAMES", "metrics:listener");
        }
        "duplicate" => {
            command
                .env("LISTEN_PID", std::process::id().to_string())
                .env("LISTEN_FDS", "2")
                .env("LISTEN_FDNAMES", "listener:listener");
        }
        "absent" => {
            command
                .env_remove("LISTEN_PID")
                .env_remove("LISTEN_FDS")
                .env_remove("LISTEN_FDNAMES");
        }
        "foreign" => {
            command
                .env(
                    "LISTEN_PID",
                    std::process::id().saturating_add(1).to_string(),
                )
                .env("LISTEN_FDS", "garbage");
        }
        _ => return Err(std::io::Error::other("unknown fixture mode")),
    }
    // exec preserves this manager's PID. Environment preparation occurs in this
    // already separate, single-threaded process, not a fork/pre_exec callback.
    Err(command.exec())
}
