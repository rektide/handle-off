//! Standard I/O is a role convention, independently usable without a manifest.
//! Borrow from caller-owned handles; never take ownership of raw FD 0/1/2.
#[cfg(unix)]
mod unix {
    use std::{
        io,
        os::fd::{AsFd, BorrowedFd, OwnedFd},
    };
    pub const STDIN: &str = "stdin";
    pub const STDOUT: &str = "stdout";
    pub const STDERR: &str = "stderr";
    #[derive(Debug)]
    pub struct Stdio<H> {
        pub stdin: H,
        pub stdout: H,
        pub stderr: H,
    }
    pub type BorrowedStdio<'a> = Stdio<BorrowedFd<'a>>;
    pub type OwnedStdio = Stdio<OwnedFd>;
    impl<'a> BorrowedStdio<'a> {
        pub fn new(stdin: &'a impl AsFd, stdout: &'a impl AsFd, stderr: &'a impl AsFd) -> Self {
            Self {
                stdin: stdin.as_fd(),
                stdout: stdout.as_fd(),
                stderr: stderr.as_fd(),
            }
        }
    }
    impl<H: AsFd> Stdio<H> {
        pub fn borrow(&self) -> BorrowedStdio<'_> {
            Stdio {
                stdin: self.stdin.as_fd(),
                stdout: self.stdout.as_fd(),
                stderr: self.stderr.as_fd(),
            }
        }
        /// Duplicates share file offsets and status flags with the caller's handles.
        pub fn try_clone_to_owned(&self) -> io::Result<OwnedStdio> {
            Ok(Stdio {
                stdin: self.stdin.as_fd().try_clone_to_owned()?,
                stdout: self.stdout.as_fd().try_clone_to_owned()?,
                stderr: self.stderr.as_fd().try_clone_to_owned()?,
            })
        }
    }
}
#[cfg(unix)]
pub use unix::*;
