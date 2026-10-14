#![cfg(unix)]
use handle_off_unix_rights::{MAX_FDS, MAX_PAYLOAD, ReceiveLimits, receive, send};
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsFd, OwnedFd},
        unix::net::{UnixDatagram, UnixStream},
    },
    time::Duration,
};

fn limits() -> ReceiveLimits {
    ReceiveLimits {
        max_payload: 1024,
        max_fds: 8,
    }
}
fn pair() -> (UnixDatagram, UnixDatagram) {
    let (sender, receiver) = UnixDatagram::pair().unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    (sender, receiver)
}
fn assert_cloexec(fd: &OwnedFd) {
    assert!(
        rustix::io::fcntl_getfd(fd)
            .unwrap()
            .contains(rustix::io::FdFlags::CLOEXEC)
    );
}

#[test]
fn multiple_pipe_socket_and_file_references_with_one_payload() {
    let (sender, receiver) = pair();
    let (read, write) = rustix::pipe::pipe().unwrap();
    let mut pipe_writer = File::from(write);
    let (socket, mut peer) = UnixStream::pair().unwrap();
    let file = File::open("/dev/null").unwrap();
    send(
        &sender,
        b"pipe,socket,file\0\xff",
        &[read.as_fd(), socket.as_fd(), file.as_fd()],
    )
    .unwrap();
    let mut message = receive(&receiver, limits()).unwrap();
    assert_eq!(message.payload, b"pipe,socket,file\0\xff");
    assert_eq!(message.descriptors.len(), 3);
    for fd in &message.descriptors {
        assert_cloexec(fd);
    }
    // Send only borrowed references. Originals are still live after receiving.
    assert!(rustix::io::fcntl_getfd(&read).is_ok());
    assert!(rustix::io::fcntl_getfd(&socket).is_ok());
    assert!(rustix::io::fcntl_getfd(&file).is_ok());
    drop(read);
    drop(socket);
    drop(file);
    drop(sender);
    let mut pipe_reader = File::from(message.descriptors.remove(0));
    let mut socket = UnixStream::from(message.descriptors.remove(0));
    let mut file = File::from(message.descriptors.remove(0));
    pipe_writer.write_all(b"pipe").unwrap();
    peer.write_all(b"sock").unwrap();
    let mut bytes = [0; 4];
    pipe_reader.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"pipe");
    socket.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"sock");
    assert_eq!(file.read(&mut bytes).unwrap(), 0);
}

fn rejected_message_releases_every_copy(
    payload: &[u8],
    copies: usize,
    receive_limits: ReceiveLimits,
) {
    let (sender, receiver) = pair();
    let (socket, mut peer) = UnixStream::pair().unwrap();
    let sentinel = File::open("/dev/null").unwrap();
    let descriptors: Vec<_> = (0..copies).map(|_| socket.as_fd()).collect();
    send(&sender, payload, &descriptors).unwrap();
    assert_eq!(
        receive(&receiver, receive_limits).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    // No global descriptor count assumptions: peer EOF directly proves that no
    // received copies of this unique socket survive the rejected message.
    assert!(rustix::io::fcntl_getfd(&socket).is_ok());
    assert!(rustix::io::fcntl_getfd(&sentinel).is_ok());
    drop(socket);
    peer.set_nonblocking(true).unwrap();
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    send(&sender, b"next", &[]).unwrap();
    assert_eq!(receive(&receiver, limits()).unwrap().payload, b"next");
}
#[test]
fn payload_truncation_closes_all_acquired_rights() {
    rejected_message_releases_every_copy(
        b"oversized",
        3,
        ReceiveLimits {
            max_payload: 2,
            max_fds: 8,
        },
    );
}
#[test]
fn control_truncation_closes_partial_and_kernel_discarded_rights() {
    rejected_message_releases_every_copy(
        b"rights",
        8,
        ReceiveLimits {
            max_payload: 64,
            max_fds: 1,
        },
    );
    rejected_message_releases_every_copy(
        b"rights",
        1,
        ReceiveLimits {
            max_payload: 64,
            max_fds: 0,
        },
    );
}
#[test]
fn padding_cannot_bypass_descriptor_count_limit() {
    // On Linux a one-FD CMSG allocation can hold two FDs due to alignment.
    rejected_message_releases_every_copy(
        b"rights",
        2,
        ReceiveLimits {
            max_payload: 64,
            max_fds: 1,
        },
    );
}
#[test]
fn empty_peer_payload_is_rejected_and_received_rights_closed() {
    use rustix::net::{SendAncillaryBuffer, SendAncillaryMessage, SendFlags};
    let (sender, receiver) = pair();
    let (socket, mut peer) = UnixStream::pair().unwrap();
    let fds = [socket.as_fd()];
    let mut bytes = [std::mem::MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut control = SendAncillaryBuffer::new(&mut bytes);
    assert!(control.push(SendAncillaryMessage::ScmRights(&fds)));
    rustix::net::sendmsg(
        &sender,
        &[io::IoSlice::new(b"")],
        &mut control,
        SendFlags::empty(),
    )
    .unwrap();
    assert_eq!(
        receive(&receiver, limits()).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    drop(socket);
    peer.set_nonblocking(true).unwrap();
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
}
#[test]
fn invalid_limits_and_wrong_socket_types_do_not_close_caller_fds() {
    let (sender, receiver) = pair();
    let file = File::open("/dev/null").unwrap();
    assert!(send(&sender, b"", &[file.as_fd()]).is_err());
    assert!(send(&sender, &vec![0; MAX_PAYLOAD + 1], &[]).is_err());
    assert!(send(&sender, b"x", &vec![file.as_fd(); MAX_FDS + 1]).is_err());
    assert!(
        receive(
            &receiver,
            ReceiveLimits {
                max_payload: 0,
                max_fds: 1
            }
        )
        .is_err()
    );
    assert!(
        receive(
            &receiver,
            ReceiveLimits {
                max_payload: 1,
                max_fds: MAX_FDS + 1
            }
        )
        .is_err()
    );
    // Safe std conversion can wrap a stream as UnixDatagram. Runtime checking
    // prevents exposing ambiguous stream rights/partial-send behavior.
    let (stream, _peer) = UnixStream::pair().unwrap();
    let fake = UnixDatagram::from(OwnedFd::from(stream));
    assert_eq!(
        send(&fake, b"x", &[file.as_fd()]).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        receive(&fake, limits()).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(rustix::io::fcntl_getfd(&file).is_ok());
    assert!(rustix::io::fcntl_getfd(&fake).is_ok());
    send(&sender, b"ok", &[file.as_fd()]).unwrap();
    assert_eq!(receive(&receiver, limits()).unwrap().descriptors.len(), 1);
}
#[test]
fn nonblocking_and_failed_send_preserve_ownership() {
    let (sender, receiver) = pair();
    receiver.set_nonblocking(true).unwrap();
    assert_eq!(
        receive(&receiver, limits()).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    drop(receiver);
    let file = File::open("/dev/null").unwrap();
    assert!(send(&sender, b"x", &[file.as_fd()]).is_err());
    assert!(rustix::io::fcntl_getfd(&file).is_ok());
}
