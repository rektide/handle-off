#![cfg(target_os = "linux")]
use handle_off_bundle::{
    Bundle, BundleId, GenerationId, MANIFEST_VERSION, Manifest, Member, MemberId,
};
use handle_off_handles::{OwnedHandles, ResourceId};
use handle_off_state::{Extension, Payload, StateFormat};
use handle_off_state_memfd::{BorrowedImage, OwnedImage};
use handle_off_state_rkyv::{Limits, decode_manifest, encode_manifest};
use handle_off_unix_rights::{ReceiveLimits, receive, send};
use std::{
    os::{fd::AsFd, unix::net::UnixDatagram},
    time::Duration,
};

#[test]
fn manifest_and_multiple_independent_images_transfer_without_coupling() {
    let id = |name: &str| ResourceId::new(name).unwrap();
    let first = OwnedImage::create(b"first state", 64).unwrap();
    let second = OwnedImage::create(b"second state", 64).unwrap();
    let manifest = Manifest {
        version: MANIFEST_VERSION,
        generation: GenerationId::new("cut-1").unwrap(),
        resources: vec![id("first"), id("second")],
        bundles: ["first", "second"]
            .into_iter()
            .map(|name| Bundle {
                id: BundleId::new(name).unwrap(),
                members: vec![Member {
                    id: MemberId::new("state").unwrap(),
                    resource: id(name),
                    extensions: vec![Extension {
                        key: "checkpoint".into(),
                        format: StateFormat {
                            name: "application.state".into(),
                            version: 1,
                        },
                        required: true,
                        payload: Payload::ResourceRange {
                            resource: id(name),
                            offset: 0,
                            length: if name == "first" { 11 } else { 12 },
                        },
                    }],
                }],
                extensions: vec![],
            })
            .collect(),
        extensions: vec![],
    };
    let bytes = encode_manifest(&manifest, Limits::default()).unwrap();
    let metadata = OwnedImage::create(&bytes, 4096).unwrap();
    let (sender, receiver) = UnixDatagram::pair().unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    // This fixture supplies explicit index bindings, not a new bundle protocol.
    send(
        &sender,
        b"metadata,second,first",
        &[metadata.as_fd(), second.as_fd(), first.as_fd()],
    )
    .unwrap();
    drop(metadata);
    drop(first);
    drop(second);
    let mut message = receive(
        &receiver,
        ReceiveLimits {
            max_payload: 64,
            max_fds: 3,
        },
    )
    .unwrap();
    assert_eq!(message.payload, b"metadata,second,first");
    let metadata = OwnedImage::adopt(message.descriptors.remove(0), 4096).unwrap();
    let restored = decode_manifest(&metadata.read_all().unwrap(), Limits::default()).unwrap();
    assert_eq!(restored, manifest);
    let mut handles = OwnedHandles::default();
    handles
        .insert(id("second"), message.descriptors.remove(0))
        .unwrap();
    handles
        .insert(id("first"), message.descriptors.remove(0))
        .unwrap();
    restored.validate_handles(&handles).unwrap();
    for bundle in &restored.bundles {
        let Payload::ResourceRange {
            resource,
            offset,
            length,
        } = &bundle.members[0].extensions[0].payload
        else {
            panic!("fixture must reference carrier");
        };
        let image =
            BorrowedImage::from_borrowed(handles.get(resource).unwrap().as_fd(), 64).unwrap();
        let expected: &[u8] = if bundle.id.as_str() == "first" {
            b"first state"
        } else {
            b"second state"
        };
        assert_eq!(image.read_range(*offset, *length).unwrap(), expected);
    }
}
