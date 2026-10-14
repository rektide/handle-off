#![cfg(unix)]
use handle_off_bundle::*;
use handle_off_handles::{OwnedHandles, ResourceId};
use handle_off_state::{Extension, Payload, StateFormat};
use std::{
    io::{Read, Seek, SeekFrom, Write},
    os::fd::OwnedFd,
};

fn resource(name: &str) -> ResourceId {
    ResourceId::new(name).unwrap()
}
fn extension(key: &str, payload: Payload) -> Extension {
    Extension {
        key: key.into(),
        format: StateFormat {
            name: "example.private".into(),
            version: 1,
        },
        required: true,
        payload,
    }
}
fn manifest() -> Manifest {
    Manifest {
        version: MANIFEST_VERSION,
        generation: GenerationId::new("cut-1").unwrap(),
        resources: vec![resource("output"), resource("image-a"), resource("image-b")],
        bundles: ["stdio", "audit"]
            .into_iter()
            .map(|name| Bundle {
                id: BundleId::new(name).unwrap(),
                members: vec![Member {
                    id: MemberId::new("output").unwrap(),
                    resource: resource("output"),
                    extensions: vec![extension("writer", Payload::Inline(vec![0xff, 0]))],
                }],
                extensions: vec![extension(
                    "bundle-image",
                    Payload::ResourceRange {
                        resource: resource("image-a"),
                        offset: 4,
                        length: 8,
                    },
                )],
            })
            .collect(),
        extensions: vec![extension(
            "application",
            Payload::ResourceRange {
                resource: resource("image-b"),
                offset: 0,
                length: 20,
            },
        )],
    }
}
fn handles() -> OwnedHandles {
    let mut handles = OwnedHandles::default();
    // Deliberately different inventory order from the manifest.
    for name in ["image-b", "output", "image-a"] {
        handles
            .insert(
                resource(name),
                std::fs::File::open("/dev/null").unwrap().into(),
            )
            .unwrap();
    }
    handles
}
#[test]
fn multiple_bundles_share_declared_resources_and_state_carriers() {
    let manifest = manifest();
    let handles = handles();
    manifest.validate_handles(&handles.borrow()).unwrap();
    assert_eq!(
        manifest.bundles[0].members[0].resource,
        manifest.bundles[1].members[0].resource
    );
    // Identical scoped member names across distinct bundles are valid.
    assert_eq!(
        manifest.bundles[0].members[0].id,
        manifest.bundles[1].members[0].id
    );
}
#[test]
fn membership_rejects_missing_extra_and_duplicate_associations() {
    let mut manifest = manifest();
    let mut table = handles();
    let removed = table.remove(&resource("image-a")).unwrap();
    assert_eq!(
        manifest.validate_handles(&table),
        Err(ManifestError::HandleMembership)
    );
    table.insert(resource("image-a"), removed).unwrap();
    table
        .insert(
            resource("extra"),
            std::fs::File::open("/dev/null").unwrap().into(),
        )
        .unwrap();
    assert_eq!(
        manifest.validate_handles(&table),
        Err(ManifestError::HandleMembership)
    );
    manifest.resources.push(resource("output"));
    assert_eq!(
        manifest.validate(),
        Err(ManifestError::DuplicateResource(resource("output")))
    );
    manifest.resources.pop();
    manifest.bundles.push(manifest.bundles[0].clone());
    assert!(matches!(
        manifest.validate(),
        Err(ManifestError::DuplicateBundle(_))
    ));
    manifest.bundles.pop();
    let member = manifest.bundles[0].members[0].clone();
    manifest.bundles[0].members.push(member);
    assert!(matches!(
        manifest.validate(),
        Err(ManifestError::DuplicateMember { .. })
    ));
}
#[test]
fn state_associations_are_complete_and_extensions_remain_opaque() {
    let mut manifest = manifest();
    manifest.extensions[0].payload = Payload::ResourceRange {
        resource: resource("absent"),
        offset: 0,
        length: 1,
    };
    assert_eq!(
        manifest.validate(),
        Err(ManifestError::MissingResource(resource("absent")))
    );
    manifest.extensions[0].payload = Payload::Inline(vec![0xff]);
    assert_eq!(
        manifest.validate(),
        Err(ManifestError::UnreferencedResource(resource("image-b")))
    );
    manifest.resources.retain(|id| id != &resource("image-b"));
    manifest.extensions[0].required = false;
    manifest.extensions[0].format.version = 999;
    // Structural validation cannot decide whether an application understands a format.
    manifest.validate().unwrap();
    manifest.extensions.push(manifest.extensions[0].clone());
    assert!(matches!(
        manifest.validate(),
        Err(ManifestError::DuplicateExtension(_))
    ));
    manifest.extensions.pop();
    manifest.version = 999;
    assert_eq!(
        manifest.validate(),
        Err(ManifestError::UnsupportedVersion(999))
    );
}
#[test]
fn explicit_duplication_shares_offsets_not_descriptor_identity() {
    let path = std::env::temp_dir().join(format!("handle-off-offset-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    file.write_all(b"abcd").unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut table = OwnedHandles::default();
    table.insert(resource("file"), OwnedFd::from(file)).unwrap();
    let mut duplicate = table.try_clone_to_owned().unwrap();
    let mut first = std::fs::File::from(table.remove(&resource("file")).unwrap());
    let mut second = std::fs::File::from(duplicate.remove(&resource("file")).unwrap());
    let mut byte = [0];
    first.read_exact(&mut byte).unwrap();
    assert_eq!(&byte, b"a");
    second.read_exact(&mut byte).unwrap();
    assert_eq!(&byte, b"b");
    drop(first);
    second.read_exact(&mut byte).unwrap();
    assert_eq!(&byte, b"c");
}
