#![cfg(unix)]
use handle_off_bundle::{
    Bundle, BundleId, GenerationId, MANIFEST_VERSION, Manifest, Member, MemberId,
};
use handle_off_handles::{BorrowedHandles, ResourceId};
use handle_off_handover::{Authority, Evidence, ParkingController, RelinquishOutcome};
use handle_off_handover_direct::{Correlation, Sender};
use handle_off_state::{Extension, Payload, StateFormat};
use handle_off_transport_unix::{Endpoint, Limits};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::{
        fd::AsFd,
        unix::{
            fs::DirBuilderExt,
            net::{UnixDatagram, UnixStream},
        },
    },
    process::{Child, Command, Stdio},
    time::Duration,
};
struct Guard(Child);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn run(abort: bool) {
    let root = std::env::temp_dir().join(format!(
        "handle-off-direct-handover-{}-{abort}",
        std::process::id()
    ));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let data_parent = root.join("data-old");
    let data_child = root.join("data-new");
    let control_parent = root.join("ctl-old");
    let control_child = root.join("ctl-new");
    let data = UnixDatagram::bind(&data_parent).unwrap();
    data.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let control = UnixDatagram::bind(&control_parent).unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_handle-off-direct-handover-example"))
        .args([&data_child, &data_parent, &control_child, &control_parent])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = Guard(child);
    let mut stdout = BufReader::new(child.0.stdout.take().unwrap());
    let mut listening = String::new();
    stdout.read_line(&mut listening).unwrap();
    assert_eq!(listening, "listening\n");
    data.connect(&data_child).unwrap();
    control.connect(&control_child).unwrap();
    let mut endpoint = Endpoint::new(data, Limits::default()).unwrap();
    let generation = GenerationId::new("direct-demo").unwrap();
    let correlation =
        Correlation::new([0x42; 16], generation.clone(), endpoint.next_offer_id()).unwrap();
    let mut sender = Sender::new(control, correlation, Duration::from_secs(3)).unwrap();
    let (mut old, mut peer) = UnixStream::pair().unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let full = b"abcdefghij\n";
    old.write_all(&full[..3]).unwrap();
    let mut prefix = [0; 3];
    peer.read_exact(&mut prefix).unwrap();
    assert_eq!(&prefix, b"abc");
    let resource = ResourceId::new("live-stream").unwrap();
    let mut image = 3u64.to_le_bytes().to_vec();
    image.extend_from_slice(full);
    let manifest = Manifest {
        version: MANIFEST_VERSION,
        generation: generation.clone(),
        resources: vec![resource.clone()],
        bundles: vec![Bundle {
            id: BundleId::new("application").unwrap(),
            members: vec![Member {
                id: MemberId::new("stream").unwrap(),
                resource: resource.clone(),
                extensions: vec![Extension {
                    key: "pending".into(),
                    format: StateFormat {
                        name: "example.pending-write".into(),
                        version: 1,
                    },
                    required: true,
                    payload: Payload::Inline(image),
                }],
            }],
            extensions: vec![],
        }],
        extensions: vec![],
    };
    let evidence = |value| Evidence::new(generation.clone(), value);
    let mut controller = ParkingController::new_parked(generation.clone());
    let mut attempt = controller.attempt().unwrap();
    let mut receipt = None;
    {
        let mut handles = BorrowedHandles::default();
        handles.insert(resource, old.as_fd()).unwrap();
        attempt
            .publish(
                evidence("connected data/control correlation retained"),
                || match endpoint.send(&manifest, &handles) {
                    Ok(value) => {
                        receipt = Some(value);
                        Ok(evidence("custody delivery only"))
                    }
                    Err(_) => Err(evidence("delivery uncertain")),
                },
            )
            .unwrap();
    }
    attempt
        .verify_custody(evidence("wait dormant readiness"), || {
            sender
                .wait_ready(receipt.as_ref().unwrap())
                .map(|_| evidence("validated dormant successor"))
                .map_err(|_| evidence("readiness uncertain"))
        })
        .unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(75)))
        .unwrap();
    let dormant_error = peer.read(&mut [0]).unwrap_err();
    assert!(
        matches!(
            dormant_error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ),
        "successor must emit NOTHING before release"
    );
    peer.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    if abort {
        let permit = attempt
            .abort(evidence("retire ALL successor copies"), || {
                sender
                    .abort()
                    .map(|_| evidence("retirement acknowledged"))
                    .map_err(|_| evidence("retirement ambiguous"))
            })
            .unwrap();
        assert_eq!(permit.generation(), &generation);
        old.write_all(&full[3..]).unwrap(); // Explicit application use of the old resume approval.
        drop(permit);
        assert_eq!(attempt.status().authority, Authority::OldResumeIssued);
    } else {
        let mut old_retired = false;
        attempt
            .relinquish(evidence("irrevocably disable old writer"), || {
                old_retired = true;
                RelinquishOutcome::Relinquished(evidence("old writer revoked"))
            })
            .unwrap();
        attempt
            .release(
                evidence("release only after local irreversible boundary"),
                |grant| {
                    assert!(old_retired);
                    sender
                        .release(grant)
                        .map(|_| evidence("release receipt, not completion"))
                        .map_err(|_| evidence("release ambiguous"))
                },
            )
            .unwrap();
        assert_eq!(attempt.status().authority, Authority::NewReleaseIssued);
        // No old writer I/O after its authority is relinquished.
    }
    let mut suffix = [0; 8];
    peer.read_exact(&mut suffix).unwrap();
    assert_eq!(&suffix, b"defghij\n");
    if !abort {
        peer.write_all(b"ping").unwrap();
        let mut reply = [0; 4];
        peer.read_exact(&mut reply).unwrap();
        assert_eq!(&reply, b"pong");
    }
    let status = child.0.wait().unwrap();
    let mut stderr = String::new();
    child
        .0
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(status.success(), "{stderr}");
    std::fs::remove_file(data_parent).unwrap();
    std::fs::remove_file(control_parent).unwrap();
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn successor_continues_same_live_connection_and_state_only_after_release() {
    run(false);
}
#[test]
fn abort_retires_application_and_delivery_copies_before_old_continues_stream() {
    run(true);
}
