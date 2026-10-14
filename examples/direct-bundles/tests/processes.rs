#![cfg(target_os = "linux")]
use handle_off_bundle::{
    Bundle, BundleId, GenerationId, MANIFEST_VERSION, Manifest, Member, MemberId,
};
use handle_off_handles::{OwnedHandles, ResourceId};
use handle_off_state::{Extension, Payload, StateFormat};
use handle_off_state_memfd::{BorrowedImage, OwnedImage};
use handle_off_transport_unix::{Endpoint, Limits, SendPhase};
use std::{
    io::{BufRead, BufReader, Read},
    os::{
        fd::AsFd,
        unix::{fs::DirBuilderExt, net::UnixDatagram},
    },
    process::{Command, Stdio},
    time::Duration,
};

#[test]
fn multiple_bundles_and_images_cross_process_boundary_with_explicit_acceptance() {
    let root = std::env::temp_dir().join(format!("handle-off-bundles-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let local = root.join("sender");
    let remote = root.join("receiver");
    let socket = UnixDatagram::bind(&local).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_handle-off-direct-bundles-example"))
        .arg(&remote)
        .arg(&local)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    output.read_line(&mut ready).unwrap();
    assert_eq!(ready, "ready\n");
    socket.connect(&remote).unwrap();
    let mut endpoint = Endpoint::new(socket, Limits::default()).unwrap();
    let id = |name: &str| ResourceId::new(name).unwrap();
    let images = [
        ("image-b", b"second\0\xff\n".as_slice()),
        ("image-a", b"first\0\xff\n".as_slice()),
    ];
    let mut handles = OwnedHandles::default();
    for (name, bytes) in images {
        handles
            .insert(id(name), OwnedImage::create(bytes, 1024).unwrap().into_fd())
            .unwrap();
    }
    // Manifest order deliberately differs from HandleTable's sorted iteration.
    let manifest = Manifest {
        version: MANIFEST_VERSION,
        generation: GenerationId::new("generation-process-1").unwrap(),
        resources: vec![id("image-b"), id("image-a")],
        bundles: images
            .into_iter()
            .map(|(name, bytes)| Bundle {
                id: BundleId::new(name).unwrap(),
                members: vec![Member {
                    id: MemberId::new("image").unwrap(),
                    resource: id(name),
                    extensions: vec![Extension {
                        key: "checkpoint".into(),
                        format: StateFormat {
                            name: "example.bytes".into(),
                            version: 1,
                        },
                        required: true,
                        payload: Payload::ResourceRange {
                            resource: id(name),
                            offset: 0,
                            length: bytes.len() as u64,
                        },
                    }],
                }],
                extensions: vec![],
            })
            .collect(),
        extensions: vec![],
    };
    let receipt = endpoint.send(&manifest, &handles.borrow()).unwrap();
    assert_eq!(receipt.offer_id, 1);
    assert_eq!(receipt.generation, manifest.generation);
    assert!(!endpoint.is_poisoned());
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    let status = child.wait_with_output().unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    assert_eq!(bytes, b"second\0\xff\nfirst\0\xff\n");
    // A receipt never revokes original references; retained images still work.
    for (name, expected) in images {
        let image =
            BorrowedImage::from_borrowed(handles.get(&id(name)).unwrap().as_fd(), 1024).unwrap();
        assert_eq!(image.read_all().unwrap(), expected);
    }
    std::fs::remove_file(local).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[test]
fn required_generation_and_bundle_extensions_are_rejected_before_acceptance() {
    for (case, at_generation) in [("generation", true), ("bundle", false)] {
        let root = std::env::temp_dir().join(format!(
            "handle-off-bundles-ext-{case}-{}",
            std::process::id()
        ));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        let local = root.join("sender");
        let remote = root.join("receiver");
        let socket = UnixDatagram::bind(&local).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_handle-off-direct-bundles-example"))
            .arg(&remote)
            .arg(&local)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let mut ready = String::new();
        output.read_line(&mut ready).unwrap();
        assert_eq!(ready, "ready\n");
        socket.connect(&remote).unwrap();
        let mut endpoint = Endpoint::new(socket, Limits::default()).unwrap();
        let id = |name: &str| ResourceId::new(name).unwrap();
        // A required extension in a format this sample has never heard of.
        let required = Extension {
            key: "future".into(),
            format: StateFormat {
                name: "example.unknown".into(),
                version: 1,
            },
            required: true,
            payload: Payload::Inline(vec![0]),
        };
        let mut handles = OwnedHandles::default();
        let mut bundles = Vec::new();
        for (name, bytes) in [
            ("image-b", b"second\0\xff\n".as_slice()),
            ("image-a", b"first\0\xff\n".as_slice()),
        ] {
            handles
                .insert(id(name), OwnedImage::create(bytes, 1024).unwrap().into_fd())
                .unwrap();
            bundles.push(Bundle {
                id: BundleId::new(name).unwrap(),
                members: vec![Member {
                    id: MemberId::new("image").unwrap(),
                    resource: id(name),
                    extensions: vec![Extension {
                        key: "checkpoint".into(),
                        format: StateFormat {
                            name: "example.bytes".into(),
                            version: 1,
                        },
                        required: true,
                        payload: Payload::ResourceRange {
                            resource: id(name),
                            offset: 0,
                            length: bytes.len() as u64,
                        },
                    }],
                }],
                extensions: if !at_generation && name == "image-a" {
                    vec![required.clone()]
                } else {
                    vec![]
                },
            });
        }
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            generation: GenerationId::new(format!("generation-ext-{case}")).unwrap(),
            resources: vec![id("image-b"), id("image-a")],
            bundles,
            extensions: if at_generation {
                vec![required]
            } else {
                vec![]
            },
        };
        // Rejection happens before acceptance: no correlated ACK is ever
        // sent, so the sender's outcome is indeterminate (refused or timed
        // out), never a receipt.
        let error = endpoint.send(&manifest, &handles.borrow()).unwrap_err();
        assert_eq!(error.phase, SendPhase::Indeterminate, "{case}: {error}");
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success(), "{case}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let expected = if at_generation {
            "generation-scope"
        } else {
            "bundle-scope"
        };
        assert!(stderr.contains(expected), "{case}: {stderr}");
        // The receiver errors before retiring its own path; clean up here.
        std::fs::remove_file(local).unwrap();
        std::fs::remove_file(remote).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
