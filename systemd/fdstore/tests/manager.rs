#![cfg(unix)]
use handle_off_systemd_fdstore::{FdName, PollPolicy, Store, Verification};
use handle_off_systemd_notify::{BarrierOutcome, Notifier, NotifySocket, Submission};
use handle_off_unix_rights::{ReceiveLimits, receive};
use std::{
    net::TcpListener,
    os::{
        fd::AsFd,
        unix::{fs::DirBuilderExt, net::UnixDatagram},
    },
    path::PathBuf,
    time::Duration,
};

fn manager(name: &str) -> (PathBuf, UnixDatagram, Notifier) {
    let root =
        std::env::temp_dir().join(format!("handle-off-fdstore-{}-{name}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let path = root.join("notify");
    let manager = UnixDatagram::bind(&path).unwrap();
    manager
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let notifier = Notifier::new(
        NotifySocket::from_value(path.as_os_str()).unwrap(),
        Duration::from_secs(1),
    )
    .unwrap();
    (path, manager, notifier)
}
fn limits() -> ReceiveLimits {
    ReceiveLimits {
        max_payload: 1024,
        max_fds: 8,
    }
}

#[test]
fn named_multi_fd_offers_preserve_originals_and_explicit_verification() {
    let (path, manager, notifier) = manager("multi");
    let store = Store::new(&notifier);
    let first = TcpListener::bind("127.0.0.1:0").unwrap();
    let second = TcpListener::bind("127.0.0.1:0").unwrap();
    let expected = [first.local_addr().unwrap(), second.local_addr().unwrap()];
    let name = FdName::new("same tag = all").unwrap();
    let receipt = store
        .offer(
            &name,
            &[first.as_fd(), second.as_fd()],
            PollPolicy::Disabled,
        )
        .unwrap();
    assert_eq!(receipt.submission, Submission::Submitted);
    assert_eq!(receipt.descriptors.len(), 2);
    let mut message = receive(&manager, limits()).unwrap();
    assert_eq!(
        message.payload,
        b"FDSTORE=1\nFDNAME=same tag = all\nFDPOLL=0"
    );
    let stored_first = TcpListener::from(message.descriptors.remove(0));
    let stored_second = TcpListener::from(message.descriptors.remove(0));
    let verified = receipt
        .verify_with::<std::io::Error>(|receipt| {
            assert_eq!(receipt.name.as_str(), "same tag = all");
            assert_eq!(
                [stored_first.local_addr()?, stored_second.local_addr()?],
                expected
            );
            Ok(Verification::Confirmed)
        })
        .unwrap();
    assert_eq!(verified, Verification::Confirmed);
    drop(receipt);
    drop(first);
    drop(second);
    assert_eq!(stored_first.local_addr().unwrap(), expected[0]);
    assert_eq!(stored_second.local_addr().unwrap(), expected[1]);
    // Same tag may be submitted again. No artificial uniqueness or pruning policy.
    let again = store
        .offer(&name, &[stored_first.as_fd()], PollPolicy::ManagerDefault)
        .unwrap();
    assert_eq!(
        again
            .verify_with::<std::io::Error>(|_| Ok(Verification::Unsupported))
            .unwrap(),
        Verification::Unsupported
    );
    let message = receive(&manager, limits()).unwrap();
    assert_eq!(message.payload, b"FDSTORE=1\nFDNAME=same tag = all");
    let removal = store.remove(&name).unwrap();
    assert_eq!(removal.submission, Submission::Submitted);
    let message = receive(&manager, limits()).unwrap();
    assert_eq!(message.payload, b"FDSTOREREMOVE=1\nFDNAME=same tag = all");
    assert!(message.descriptors.is_empty());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}
#[test]
fn no_manager_is_not_stored_or_verified_and_invalid_offer_retains_owner() {
    let notifier = Notifier::new(None, Duration::from_secs(1)).unwrap();
    let store = Store::new(&notifier);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let name = FdName::new("listener").unwrap();
    let receipt = store
        .offer(&name, &[listener.as_fd()], PollPolicy::ManagerDefault)
        .unwrap();
    assert_eq!(receipt.submission, Submission::NoManager);
    assert_eq!(
        receipt
            .verify_with::<()>(|_| panic!("absent manager must bypass verifier"))
            .unwrap(),
        Verification::NoManager
    );
    let removal = store.remove(&name).unwrap();
    assert_eq!(
        removal
            .verify_with::<()>(|_| panic!("absent manager must bypass verifier"))
            .unwrap(),
        Verification::NoManager
    );
    assert!(store.offer(&name, &[], PollPolicy::Disabled).is_err());
    assert_eq!(listener.local_addr().unwrap(), address);
}

#[test]
fn barrier_pipe_closure_does_not_turn_rejected_storage_into_acceptance() {
    let (path, manager, notifier) = manager("reject-store");
    let worker = std::thread::spawn(move || {
        let offered = receive(&manager, limits()).unwrap();
        assert_eq!(offered.payload, b"FDSTORE=1\nFDNAME=listener");
        drop(offered); // A configured manager may reject all offered references.
        let barrier = receive(&manager, limits()).unwrap();
        assert_eq!(barrier.payload, b"BARRIER=1");
        drop(barrier); // Ordering notification still receives its pipe-close signal.
    });
    let store = Store::new(&notifier);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let receipt = store
        .offer(
            &FdName::new("listener").unwrap(),
            &[listener.as_fd()],
            PollPolicy::ManagerDefault,
        )
        .unwrap();
    assert_eq!(
        notifier.barrier(Duration::from_secs(1)).unwrap(),
        BarrierOutcome::PipeClosed
    );
    worker.join().unwrap();
    assert_eq!(
        receipt
            .verify_with::<()>(|_| Ok(Verification::Rejected))
            .unwrap(),
        Verification::Rejected
    );
    assert!(listener.local_addr().is_ok());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}
#[cfg(target_os = "linux")]
#[test]
fn abstract_fdstore_endpoint_carries_actual_listener() {
    use std::os::linux::net::SocketAddrExt;
    let name = format!("handle-off-fdstore-abstract-{}", std::process::id());
    let address = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes()).unwrap();
    let manager = UnixDatagram::bind_addr(&address).unwrap();
    manager
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let endpoint = NotifySocket::from_value(std::ffi::OsStr::new(&format!("@{name}"))).unwrap();
    let notifier = Notifier::new(endpoint, Duration::from_secs(1)).unwrap();
    let store = Store::new(&notifier);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    store
        .offer(
            &FdName::new("listener").unwrap(),
            &[listener.as_fd()],
            PollPolicy::ManagerDefault,
        )
        .unwrap();
    let mut message = receive(&manager, limits()).unwrap();
    assert_eq!(message.payload, b"FDSTORE=1\nFDNAME=listener");
    assert_eq!(
        TcpListener::from(message.descriptors.remove(0))
            .local_addr()
            .unwrap(),
        listener.local_addr().unwrap()
    );
}
