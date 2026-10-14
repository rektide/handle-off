use crate::data::Error;
use std::{
    io::Read,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
struct Guard {
    child: Child,
    group: rustix::process::Pid,
    finished: bool,
}
impl Drop for Guard {
    fn drop(&mut self) {
        if !self.finished {
            let _ = rustix::process::kill_process_group(self.group, rustix::process::Signal::KILL);
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn drain(
    pipe: &mut impl Read,
    bytes: &mut Vec<u8>,
    other_len: usize,
    maximum: usize,
    deadline: Instant,
) -> Result<bool, Error> {
    let mut buffer = [0; 8192];
    loop {
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        match pipe.read(&mut buffer) {
            Ok(0) => return Ok(true),
            Ok(count) => {
                if count
                    > maximum
                        .saturating_sub(bytes.len())
                        .saturating_sub(other_len)
                {
                    return Err(Error::OutputLimit);
                }
                bytes.extend_from_slice(&buffer[..count]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(false),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
}
pub(crate) fn run(
    program: &Path,
    arguments: &[String],
    deadline: Instant,
    maximum: usize,
) -> Result<Vec<u8>, Error> {
    if Instant::now() >= deadline {
        return Err(Error::Timeout);
    }
    let child = Command::new(program)
        .args(arguments)
        .env("LC_ALL", "C")
        .env("SYSTEMD_COLORS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let group = rustix::process::Pid::from_child(&child);
    let mut guard = Guard {
        child,
        group,
        finished: false,
    };
    let mut stdout = guard
        .child
        .stdout
        .take()
        .ok_or(Error::InvalidReply("missing command stdout"))?;
    let mut stderr = guard
        .child
        .stderr
        .take()
        .ok_or(Error::InvalidReply("missing command stderr"))?;
    for pipe in [
        &stdout as &dyn std::os::fd::AsFd,
        &stderr as &dyn std::os::fd::AsFd,
    ] {
        let flags = rustix::fs::fcntl_getfl(pipe)?;
        rustix::fs::fcntl_setfl(pipe, flags | rustix::fs::OFlags::NONBLOCK)?;
    }
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (mut out_done, mut err_done) = (false, false);
    loop {
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        if !out_done {
            out_done = drain(&mut stdout, &mut out, err.len(), maximum, deadline)?;
        }
        if !err_done {
            err_done = drain(&mut stderr, &mut err, out.len(), maximum, deadline)?;
        }
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(Error::Timeout)?;
        if out_done && err_done {
            if let Some(status) = guard.child.try_wait()? {
                guard.finished = true;
                if status.success() {
                    return Ok(out);
                }
                let stderr = String::from_utf8_lossy(&err).into_owned();
                if [
                    "org.freedesktop.DBus.Error.AccessDenied",
                    "Access denied",
                    "Permission denied",
                    "Operation not permitted",
                    "Interactive authentication required",
                ]
                .iter()
                .any(|s| stderr.contains(s))
                {
                    return Err(Error::PermissionDenied(stderr));
                }
                if [
                    "UnknownMethod",
                    "UnknownProperty",
                    "Unknown method",
                    "Unknown property",
                ]
                .iter()
                .any(|s| stderr.contains(s))
                {
                    return Err(Error::Unsupported("manager command/member unsupported"));
                }
                if stderr.contains("File descriptor store not enabled for") {
                    return Err(Error::StoreDisabled);
                }
                return Err(Error::CommandFailed {
                    status: status.code(),
                    stderr,
                });
            }
            std::thread::sleep(remaining.min(Duration::from_millis(2)));
        } else {
            let timespec = rustix::event::Timespec::try_from(remaining)
                .map_err(|_| Error::InvalidConfiguration("query timeout cannot be represented"))?;
            let mut fds = Vec::new();
            if !out_done {
                fds.push(rustix::event::PollFd::new(
                    &stdout,
                    rustix::event::PollFlags::IN | rustix::event::PollFlags::HUP,
                ));
            }
            if !err_done {
                fds.push(rustix::event::PollFd::new(
                    &stderr,
                    rustix::event::PollFlags::IN | rustix::event::PollFlags::HUP,
                ));
            }
            match rustix::event::poll(&mut fds, Some(&timespec)) {
                Ok(0) => return Err(Error::Timeout),
                Ok(_) | Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
}
