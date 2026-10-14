//! Runtime-free service notifications to explicit filesystem/Linux abstract sockets.
//! No environment removal, timer, process supervisor, service launch or PID spoofing.
//! Submission is kernel enqueue only; NotifyAccess/unit attribution and manager
//! processing are separate. Linux managers obtain this process's credentials via
//! SO_PASSCRED; this client does not impersonate another PID. VSOCK is not supported.
//! A barrier sends ONLY BARRIER=1 with one pipe writer, drops its local writer, and
//! waits for the manager to close its copy. With a manager supporting BARRIER
//! (systemd >=246), this orders prior notifications; it does NOT prove FDSTORE
//! acceptance or removal. Pipe closure alone cannot detect an older manager that
//! simply ignored the command and closed its received FD. Support is caller-owned.
//! Timeouts bound send/wait operations, not hard-real-time scheduling guarantees.
#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use handle_off_unix_rights as rights;
    use std::{
        ffi::{OsStr, OsString},
        io,
        os::{
            fd::{AsFd, BorrowedFd},
            unix::{
                ffi::OsStrExt,
                net::{SocketAddr, UnixDatagram},
            },
        },
        time::{Duration, Instant},
    };
    pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;
    #[derive(Clone, Debug, Default)]
    pub struct Snapshot {
        pub notify_socket: Option<OsString>,
    }
    impl Snapshot {
        pub fn capture() -> Self {
            Self {
                notify_socket: std::env::var_os("NOTIFY_SOCKET"),
            }
        }
        pub fn endpoint(&self) -> io::Result<Option<NotifySocket>> {
            self.notify_socket
                .as_deref()
                .map(NotifySocket::from_value)
                .transpose()
                .map(Option::flatten)
        }
    }
    #[derive(Clone, Debug)]
    pub struct NotifySocket {
        address: SocketAddr,
    }
    impl NotifySocket {
        /// Empty values are absent, not successful delivery. Accepts absolute
        /// filesystem paths and Linux '@name' abstract addresses, preserving bytes.
        pub fn from_value(value: &OsStr) -> io::Result<Option<Self>> {
            let bytes = value.as_bytes();
            if bytes.is_empty() {
                return Ok(None);
            }
            if bytes.contains(&0) {
                return Err(invalid("NUL in NOTIFY_SOCKET value"));
            }
            let address = match bytes[0] {
                b'/' => SocketAddr::from_pathname(value)?,
                b'@' => {
                    if bytes.len() == 1 {
                        return Err(invalid("empty abstract socket name"));
                    }
                    #[cfg(target_os = "linux")]
                    {
                        use std::os::linux::net::SocketAddrExt;
                        SocketAddr::from_abstract_name(&bytes[1..])?
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "Linux abstract notify socket not supported on this target",
                        ));
                    }
                }
                _ => {
                    return Err(invalid(
                        "NOTIFY_SOCKET must start with '/' or '@'; VSOCK unsupported",
                    ));
                }
            };
            Ok(Some(Self { address }))
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Submission {
        NoManager,
        Submitted,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BarrierOutcome {
        NoManager,
        PipeClosed,
    }
    fn invalid(message: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, message)
    }
    fn timed_out() -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            "notification barrier timed out; processing/acceptance unknown",
        )
    }
    fn validate_timeout(timeout: Duration) -> io::Result<()> {
        if timeout.is_zero()
            || Instant::now().checked_add(timeout).is_none()
            || rustix::event::Timespec::try_from(timeout).is_err()
        {
            return Err(invalid(
                "timeout must be positive, finite and representable",
            ));
        }
        Ok(())
    }
    #[derive(Clone, Debug)]
    pub struct Notifier {
        endpoint: Option<NotifySocket>,
        send_timeout: Duration,
    }
    impl Notifier {
        pub fn new(endpoint: Option<NotifySocket>, send_timeout: Duration) -> io::Result<Self> {
            validate_timeout(send_timeout)?;
            Ok(Self {
                endpoint,
                send_timeout,
            })
        }
        pub fn from_snapshot(snapshot: &Snapshot, send_timeout: Duration) -> io::Result<Self> {
            Self::new(snapshot.endpoint()?, send_timeout)
        }
        pub fn has_manager_endpoint(&self) -> bool {
            self.endpoint.is_some()
        }
        /// Raw newline-separated assignments with borrowed optional descriptors.
        /// Calls do not retry. Errors after send attempt do not prove non-delivery;
        /// keep originals and verify custody independently before deciding recovery.
        /// Unknown assignment semantics remain caller-owned. BARRIER cannot be mixed
        /// with assignments and requires exactly one descriptor; use barrier to wait.
        pub fn send(
            &self,
            message: &str,
            descriptors: &[BorrowedFd<'_>],
        ) -> io::Result<Submission> {
            self.send_with_timeout(message, descriptors, self.send_timeout)
        }
        fn send_with_timeout(
            &self,
            message: &str,
            descriptors: &[BorrowedFd<'_>],
            timeout: Duration,
        ) -> io::Result<Submission> {
            if message.is_empty()
                || message.len() > MAX_MESSAGE_BYTES
                || message.contains(['\0', '\r'])
                || descriptors.len() > rights::MAX_FDS
            {
                return Err(invalid(
                    "invalid notification size, bytes or descriptor count",
                ));
            }
            let mut has_barrier = false;
            for assignment in message.trim_end_matches('\n').split('\n') {
                let (key, _) = assignment
                    .split_once('=')
                    .ok_or_else(|| invalid("notification assignment lacks '='"))?;
                if key.is_empty() {
                    return Err(invalid("empty notification key"));
                }
                if key == "BARRIER" {
                    has_barrier = true;
                }
            }
            if has_barrier && (message != "BARRIER=1" || descriptors.len() != 1) {
                return Err(invalid(
                    "BARRIER=1 must be its own message with exactly one FD",
                ));
            }
            let Some(endpoint) = &self.endpoint else {
                return Ok(Submission::NoManager);
            };
            let socket = UnixDatagram::unbound()?;
            socket.set_write_timeout(Some(timeout))?;
            socket.connect_addr(&endpoint.address)?;
            rights::send(&socket, message.as_bytes(), descriptors)?;
            Ok(Submission::Submitted)
        }
        pub fn ready(&self) -> io::Result<Submission> {
            self.send("READY=1", &[])
        }
        pub fn stopping(&self) -> io::Result<Submission> {
            self.send("STOPPING=1", &[])
        }
        /// Sends one heartbeat only; watchdog eligibility, cadence and scheduling
        /// are application-owned. No background timer is installed.
        pub fn watchdog(&self) -> io::Result<Submission> {
            self.send("WATCHDOG=1", &[])
        }
        pub fn status(&self, status: &str) -> io::Result<Submission> {
            if status.contains(['\n', '\r', '\0']) || status.len() > MAX_MESSAGE_BYTES - 7 {
                return Err(invalid("STATUS must be a bounded single-line value"));
            }
            self.send(&format!("STATUS={status}"), &[])
        }
        /// Includes submission in the relative deadline and recomputes remaining
        /// time after EINTR. PipeClosed means ordering only under a manager known to
        /// support BARRIER. It never proves store acceptance or supported capabilities.
        pub fn barrier(&self, timeout: Duration) -> io::Result<BarrierOutcome> {
            validate_timeout(timeout)?;
            if self.endpoint.is_none() {
                return Ok(BarrierOutcome::NoManager);
            }
            let deadline = Instant::now()
                .checked_add(timeout)
                .ok_or_else(|| invalid("barrier deadline overflow"))?;
            let (read, write) = rustix::pipe::pipe_with(
                rustix::pipe::PipeFlags::CLOEXEC | rustix::pipe::PipeFlags::NONBLOCK,
            )?;
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|d| !d.is_zero())
                .ok_or_else(timed_out)?;
            let submission = self.send_with_timeout(
                "BARRIER=1",
                &[write.as_fd()],
                self.send_timeout.min(remaining),
            )?;
            drop(write); // Manager's copy must be the only writer remaining.
            if submission == Submission::NoManager {
                return Ok(BarrierOutcome::NoManager);
            }
            loop {
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .filter(|d| !d.is_zero())
                    .ok_or_else(timed_out)?;
                let timespec = rustix::event::Timespec::try_from(remaining)
                    .map_err(|_| invalid("barrier timeout cannot be represented"))?;
                let mut fds = [rustix::event::PollFd::new(
                    &read,
                    rustix::event::PollFlags::IN | rustix::event::PollFlags::HUP,
                )];
                match rustix::event::poll(&mut fds, Some(&timespec)) {
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => return Err(error.into()),
                    Ok(0) => return Err(timed_out()),
                    Ok(_) => {
                        let events = fds[0].revents();
                        if events.intersects(
                            rustix::event::PollFlags::HUP | rustix::event::PollFlags::IN,
                        ) {
                            match rustix::io::read(&read, &mut [0u8]) {
                                Ok(0) => return Ok(BarrierOutcome::PipeClosed),
                                Ok(_) => {
                                    return Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        "manager wrote unexpected barrier pipe data",
                                    ));
                                }
                                Err(rustix::io::Errno::INTR | rustix::io::Errno::AGAIN) => continue,
                                Err(error) => return Err(error.into()),
                            }
                        }
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "unexpected barrier pipe event instead of manager closure",
                        ));
                    }
                }
            }
        }
    }
}
#[cfg(unix)]
pub use unix::*;
