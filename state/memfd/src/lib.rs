//! Independent Linux byte carrier: immutable sealed resources, no state codec.
//! Adoption validates required seals and length before access. All byte I/O is
//! positional; duplicated descriptors' shared offsets are untouched. Seals are
//! immutability, not authentication or application compatibility. No mmap/unsafe
//! interface is needed: reads return owned bytes and borrowing lends the handle.
#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
mod linux {
    use rustix::fs::{self, MemfdFlags, SealFlags};
    use std::{
        io,
        os::fd::{AsFd, BorrowedFd, OwnedFd},
    };
    pub const MAX_BYTES: u64 = 64 * 1024 * 1024;
    fn required_seals() -> SealFlags {
        SealFlags::SEAL | SealFlags::WRITE | SealFlags::SHRINK | SealFlags::GROW
    }
    fn invalid(message: &str) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }
    fn validate(fd: BorrowedFd<'_>, max_bytes: u64) -> io::Result<u64> {
        if max_bytes > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "image limit exceeds API ceiling",
            ));
        }
        if !fs::fcntl_get_seals(fd)?.contains(required_seals()) {
            return Err(invalid("state image requires WRITE/GROW/SHRINK/SEAL seals"));
        }
        let stat = fs::fstat(fd)?;
        let length = u64::try_from(stat.st_size).map_err(|_| invalid("negative image length"))?;
        if fs::FileType::from_raw_mode(stat.st_mode) != fs::FileType::RegularFile
            || length > max_bytes
        {
            return Err(invalid("invalid image type or length exceeds limit"));
        }
        Ok(length)
    }
    #[derive(Debug)]
    pub struct SealedImage<H> {
        handle: H,
        length: u64,
    }
    pub type OwnedImage = SealedImage<OwnedFd>;
    pub type BorrowedImage<'a> = SealedImage<BorrowedFd<'a>>;
    impl OwnedImage {
        pub fn create(bytes: &[u8], max_bytes: u64) -> io::Result<Self> {
            if max_bytes > MAX_BYTES || bytes.len() as u64 > max_bytes {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "image exceeds byte limit",
                ));
            }
            let fd = fs::memfd_create(
                c"handle-off-state",
                MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
            )?;
            let mut written = 0;
            while written < bytes.len() {
                match rustix::io::pwrite(&fd, &bytes[written..], written as u64) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::WriteZero,
                            "short image write",
                        ));
                    }
                    Ok(count) => written += count,
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => return Err(error.into()),
                }
            }
            fs::fcntl_add_seals(&fd, required_seals())?;
            Self::adopt(fd, max_bytes)
        }
        /// Consumes the supplied owner, including on validation failure. Never
        /// takes ownership of arbitrary raw descriptor numbers. Sets local CLOEXEC.
        pub fn adopt(handle: OwnedFd, max_bytes: u64) -> io::Result<Self> {
            let length = validate(handle.as_fd(), max_bytes)?;
            let flags = rustix::io::fcntl_getfd(&handle)?;
            rustix::io::fcntl_setfd(&handle, flags | rustix::io::FdFlags::CLOEXEC)?;
            Ok(Self { handle, length })
        }
        pub fn into_fd(self) -> OwnedFd {
            self.handle
        }
    }
    impl<'a> BorrowedImage<'a> {
        /// Borrows a sealed resource without changing any descriptor/status flags.
        pub fn from_borrowed(handle: BorrowedFd<'a>, max_bytes: u64) -> io::Result<Self> {
            let length = validate(handle, max_bytes)?;
            Ok(Self { handle, length })
        }
    }
    impl<H: AsFd> AsFd for SealedImage<H> {
        fn as_fd(&self) -> BorrowedFd<'_> {
            self.handle.as_fd()
        }
    }
    impl<H: AsFd> SealedImage<H> {
        pub fn len(&self) -> u64 {
            self.length
        }
        pub fn is_empty(&self) -> bool {
            self.length == 0
        }
        pub fn borrow(&self) -> BorrowedImage<'_> {
            SealedImage {
                handle: self.handle.as_fd(),
                length: self.length,
            }
        }
        pub fn try_clone_to_owned(&self) -> io::Result<OwnedImage> {
            Ok(SealedImage {
                handle: self.handle.as_fd().try_clone_to_owned()?,
                length: self.length,
            })
        }
        pub fn read_all(&self) -> io::Result<Vec<u8>> {
            self.read_range(0, self.length)
        }
        /// Checks the range against the validated immutable image, then reads at
        /// that offset without disturbing its shared seek position.
        pub fn read_range(&self, offset: u64, length: u64) -> io::Result<Vec<u8>> {
            if offset
                .checked_add(length)
                .is_none_or(|end| end > self.length)
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "image range out of bounds",
                ));
            }
            let size = usize::try_from(length)
                .map_err(|_| invalid("image range does not fit address space"))?;
            let mut bytes = vec![0; size];
            let mut read = 0;
            while read < bytes.len() {
                match rustix::io::pread(&self.handle, &mut bytes[read..], offset + read as u64) {
                    Ok(0) => {
                        return Err(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "sealed image short read",
                        ));
                    }
                    Ok(count) => read += count,
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(bytes)
        }
    }
}
#[cfg(target_os = "linux")]
pub use linux::*;
