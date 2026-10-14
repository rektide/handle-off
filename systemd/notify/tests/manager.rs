#![cfg(unix)]
use handle_off_systemd_notify::{BarrierOutcome, Notifier, NotifySocket, Snapshot, Submission};
use handle_off_unix_rights::{ReceiveLimits, receive, send};
use std::{
    ffi::{OsStr, OsString},
    io,
    os::{
        fd::AsFd,
        unix::{ffi::OsStringExt, fs::DirBuilderExt, net::UnixDatagram},
    },
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

fn fake_manager(name: &str) -> (PathBuf, UnixDatagram, Notifier) {
    let root =
        std::env::temp_dir().join(format!("handle-off-notify-{}-{name}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let path = root.join(OsString::from_vec(b"notify-\xff".to_vec()));
    let manager = UnixDatagram::bind(&path).unwrap();
    manager
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let notifier = Notifier::new(
        NotifySocket::from_value(path.as_os_str()).unwrap(),
        Duration::from_millis(100),
    )
    .unwrap();
    (path, manager, notifier)
}
fn cleanup(path: &Path) {
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}
fn limits() -> ReceiveLimits {
    ReceiveLimits {
        max_payload: 16 * 1024,
        max_fds: 8,
    }
}

#[test]
fn filesystem_notifications_and_barrier_ack_preserve_order() {
    let (path, manager, notifier) = fake_manager("ordering");
    let worker = thread::spawn(move || {
        for expected in [
            b"READY=1".as_slice(),
            b"STATUS=ready = yes",
            b"WATCHDOG=1",
            b"STOPPING=1",
        ] {
            let message = receive(&manager, limits()).unwrap();
            assert_eq!(message.payload, expected);
            assert!(message.descriptors.is_empty());
        }
        let message = receive(&manager, limits()).unwrap();
        assert_eq!(message.payload, b"BARRIER=1");
        assert_eq!(message.descriptors.len(), 1);
        drop(message); // Close the only remaining writer to acknowledge ordering.
    });
    assert_eq!(notifier.ready().unwrap(), Submission::Submitted);
    notifier.status("ready = yes").unwrap();
    notifier.watchdog().unwrap();
    notifier.stopping().unwrap();
    assert_eq!(
        notifier.barrier(Duration::from_secs(1)).unwrap(),
        BarrierOutcome::PipeClosed
    );
    worker.join().unwrap();
    cleanup(&path);
}
#[test]
fn barrier_requires_manager_writer_closure_and_rejects_unexpected_data() {
    let (path, manager, notifier) = fake_manager("timeout");
    let waiter = thread::spawn(move || notifier.barrier(Duration::from_millis(40)));
    let message = receive(&manager, limits()).unwrap();
    assert_eq!(message.payload, b"BARRIER=1");
    // Keeping the manager writer alive must prevent a premature successful barrier.
    assert_eq!(
        waiter.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    assert!(rustix::io::fcntl_getfd(&message.descriptors[0]).is_ok());
    drop(message);
    cleanup(&path);

    let (path, manager, notifier) = fake_manager("unexpected-data");
    let waiter = thread::spawn(move || notifier.barrier(Duration::from_secs(1)));
    let message = receive(&manager, limits()).unwrap();
    rustix::io::write(&message.descriptors[0], b"not an ack").unwrap();
    assert_eq!(
        waiter.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    drop(message);
    cleanup(&path);
}
#[test]
fn no_manager_invalid_values_and_malformed_barriers_are_explicit() {
    let notifier =
        Notifier::from_snapshot(&Snapshot::default(), Duration::from_millis(10)).unwrap();
    assert_eq!(notifier.ready().unwrap(), Submission::NoManager);
    assert_eq!(
        notifier.barrier(Duration::from_millis(1)).unwrap(),
        BarrierOutcome::NoManager
    );
    assert!(NotifySocket::from_value(OsStr::new("")).unwrap().is_none());
    for value in ["relative/path", "@", "vsock:1:2"] {
        assert!(NotifySocket::from_value(OsStr::new(value)).is_err());
    }
    assert!(NotifySocket::from_value(OsStr::new(&format!("/{}", "x".repeat(256)))).is_err());
    assert!(Notifier::new(None, Duration::ZERO).is_err());
    assert!(notifier.barrier(Duration::ZERO).is_err());
    assert!(notifier.status("bad\nFDSTORE=1").is_err());
    assert!(notifier.send("BARRIER=1", &[]).is_err());
    let file = std::fs::File::open("/dev/null").unwrap();
    for message in [
        "BARRIER=1\nREADY=1",
        "BARRIER=1\n",
        "BARRIER=0",
        "READY=1\0",
    ] {
        assert!(notifier.send(message, &[file.as_fd()]).is_err());
    }
    assert!(rustix::io::fcntl_getfd(&file).is_ok());
}
#[test]
fn full_manager_queue_and_missing_socket_surface_errors_without_leaking_sources() {
    let (path, manager, notifier) = fake_manager("full-queue");
    let mut full = false;
    for _ in 0..4096 {
        // A single sender can exhaust its own sndbuf before the manager queue.
        // Fresh senders exercise the receiver's queue bound instead.
        let filler = UnixDatagram::unbound().unwrap();
        filler.set_nonblocking(true).unwrap();
        filler.connect(&path).unwrap();
        match send(&filler, b"READY=1", &[]) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                full = true;
                break;
            }
            Err(error) => panic!("{error}"),
        }
    }
    assert!(full, "fixture must fill the datagram queue");
    let file = std::fs::File::open("/dev/null").unwrap();
    let error = notifier
        .send("FDSTORE=1\nFDNAME=test", &[file.as_fd()])
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    ));
    assert!(rustix::io::fcntl_getfd(&file).is_ok());
    drop(manager);
    cleanup(&path);
    assert!(notifier.ready().is_err());
}
#[cfg(target_os = "linux")]
#[test]
fn abstract_notify_socket_and_kernel_credentials_work() {
    use std::os::linux::net::SocketAddrExt;
    let name = format!("handle-off-abstract-{}", std::process::id());
    let address = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes()).unwrap();
    let manager = UnixDatagram::bind_addr(&address).unwrap();
    manager
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    rustix::net::sockopt::set_socket_passcred(&manager, true).unwrap();
    let notifier = Notifier::new(
        NotifySocket::from_value(OsStr::new(&format!("@{name}"))).unwrap(),
        Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(notifier.watchdog().unwrap(), Submission::Submitted);
    let mut payload = [0; 128];
    let mut space = [std::mem::MaybeUninit::uninit(); rustix::cmsg_space!(ScmCredentials(1))];
    let mut ancillary = rustix::net::RecvAncillaryBuffer::new(&mut space);
    let result = rustix::net::recvmsg(
        &manager,
        &mut [io::IoSliceMut::new(&mut payload)],
        &mut ancillary,
        rustix::net::RecvFlags::CMSG_CLOEXEC,
    )
    .unwrap();
    assert_eq!(&payload[..result.bytes], b"WATCHDOG=1");
    let mut credentials = false;
    for message in ancillary.drain() {
        if let rustix::net::RecvAncillaryMessage::ScmCredentials(cred) = message {
            assert_eq!(cred.pid.as_raw_pid() as u32, std::process::id());
            credentials = true;
        }
    }
    assert!(
        credentials,
        "manager receives kernel SCM_CREDENTIALS for the actual sender"
    );
}
