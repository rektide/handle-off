//! Blocking SCM_RIGHTS transfer over Unix **datagram** sockets. Send requires a
//! connected peer; receive also supports a bound, unconnected datagram socket.
//!
//! One nonempty payload and an ordered list of borrowed descriptors form one
//! datagram. Sending retains all references; successful receive owns new ones.
//! No stream/seqpacket framing, acknowledgment, authentication, bundle schema or
//! application I/O permission is implied. Descriptor duplication shares offsets
//! and status flags, but not descriptor numbers or FD_CLOEXEC.
//!
//! Receive consumes one datagram. Truncated payload/control data, an empty payload,
//! or too many descriptors is an error, discarding the entire message and closing
//! all acquired descriptors. There is no retry of the discarded datagram.
//! Kernel-valid ancillary messages are parsed by rustix; ancillary metadata other
//! than rights is ignored. Arbitrary malformed userspace CMSG buffers are not an
//! input API. Payload bytes are opaque: validate application contents after receipt
//! and drop the returned message on rejection.
//!
//! Received descriptors always have CLOEXEC before being returned. Linux, Android,
//! FreeBSD, NetBSD, OpenBSD and DragonFly use atomic MSG_CMSG_CLOEXEC. Other Unix
//! targets use fcntl afterward, with a fork/exec race: callers must coordinate
//! concurrent process creation there. Only Linux has been tested in this slice.
//! Socket nonblocking settings/timeouts are respected; no flags are changed.
#![forbid(unsafe_code)]

#[cfg(unix)]
mod unix {
    use rustix::net::{
        self, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, ReturnFlags,
        SendAncillaryBuffer, SendAncillaryMessage, SendFlags, SocketType,
    };
    use std::{
        io::{self, IoSlice, IoSliceMut},
        mem::MaybeUninit,
        os::{
            fd::{BorrowedFd, OwnedFd},
            unix::net::UnixDatagram,
        },
    };

    /// Conservative API ceiling, matching Linux's SCM_MAX_FD. Other kernels may
    /// impose smaller limits and report a syscall error. This is not a guarantee.
    pub const MAX_FDS: usize = 253;
    /// Allocation ceiling, not a promise the kernel accepts this datagram size.
    pub const MAX_PAYLOAD: usize = 1024 * 1024;

    #[derive(Clone, Copy, Debug)]
    pub struct ReceiveLimits {
        pub max_payload: usize,
        pub max_fds: usize,
    }

    /// Raw association by list order, not persistent resource or member identity.
    #[derive(Debug)]
    pub struct Message {
        pub payload: Vec<u8>,
        pub descriptors: Vec<OwnedFd>,
    }

    fn validate_socket(socket: &UnixDatagram) -> io::Result<()> {
        if net::sockopt::socket_type(socket)? != SocketType::DGRAM
            || net::getsockname(socket)?.address_family() != net::AddressFamily::UNIX
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "requires an AF_UNIX SOCK_DGRAM socket",
            ));
        }
        Ok(())
    }

    /// Send exactly one datagram to the socket's connected peer. Success means
    /// kernel submission, not application receipt or acceptance. No descriptors
    /// are consumed. On error, ownership remains with the caller. EINTR and
    /// WouldBlock are surfaced; no hidden retries or partial-send follow-ups occur.
    pub fn send(
        socket: &UnixDatagram,
        payload: &[u8],
        descriptors: &[BorrowedFd<'_>],
    ) -> io::Result<()> {
        if payload.is_empty() || payload.len() > MAX_PAYLOAD || descriptors.len() > MAX_FDS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid payload or descriptor count",
            ));
        }
        validate_socket(socket)?;
        let mut space =
            vec![MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(descriptors.len()))];
        let mut ancillary = SendAncillaryBuffer::new(&mut space);
        if !descriptors.is_empty() && !ancillary.push(SendAncillaryMessage::ScmRights(descriptors))
        {
            return Err(io::Error::other("ancillary buffer sizing failed"));
        }
        let sent = net::sendmsg(
            socket,
            &[IoSlice::new(payload)],
            &mut ancillary,
            SendFlags::empty(),
        )?;
        if sent != payload.len() {
            // Datagram writes should be atomic. Never retry a putative partial
            // send: that might deliver the same rights twice.
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "unexpected partial datagram send; delivery uncertain",
            ));
        }
        Ok(())
    }

    /// Consume one datagram, returning its payload and owned rights only if the
    /// entire message fits. This is a custody operation, not adoption permission.
    pub fn receive(socket: &UnixDatagram, limits: ReceiveLimits) -> io::Result<Message> {
        if limits.max_payload == 0 || limits.max_payload > MAX_PAYLOAD || limits.max_fds > MAX_FDS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid receive limits",
            ));
        }
        validate_socket(socket)?;
        let mut payload = vec![0; limits.max_payload];
        let control_size = if limits.max_fds == 0 {
            0
        } else {
            rustix::cmsg_space!(ScmRights(limits.max_fds))
        };
        let mut space = vec![MaybeUninit::uninit(); control_size];
        let mut ancillary = RecvAncillaryBuffer::new(&mut space);
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        ))]
        let flags = RecvFlags::CMSG_CLOEXEC;
        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        )))]
        let flags = RecvFlags::empty();
        let result = net::recvmsg(
            socket,
            &mut [IoSliceMut::new(&mut payload)],
            &mut ancillary,
            flags,
        )?;
        if result
            .flags
            .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC)
        {
            // rustix's buffer Drop drains and closes every received rights entry.
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "datagram payload or ancillary data truncated",
            ));
        }
        let mut descriptors = Vec::new();
        for message in ancillary.drain() {
            if let RecvAncillaryMessage::ScmRights(rights) = message {
                descriptors.extend(rights);
            }
        }
        // CMSG padding may allow more rights than requested even without CTRUNC.
        if result.bytes == 0 || descriptors.len() > limits.max_fds {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "empty payload or descriptor limit exceeded",
            ));
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "dragonfly"
        )))]
        for fd in &descriptors {
            let flags = rustix::io::fcntl_getfd(fd)?;
            rustix::io::fcntl_setfd(fd, flags | rustix::io::FdFlags::CLOEXEC)?;
        }
        payload.truncate(result.bytes);
        Ok(Message {
            payload,
            descriptors,
        })
    }
}
#[cfg(unix)]
pub use unix::*;
