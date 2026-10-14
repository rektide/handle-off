//! Checked, bounded codec with a private wire representation and explicit envelope.
//! Version 1 fixes rkyv 0.8.18, little-endian, 32-bit relative pointers, unaligned
//! archives and the private schema. A future incompatible format needs a new wire
//! version. Core semantic version and application extension versions are separate.
//! No checksum/authentication is provided: bytecheck validates structure, not trust.
//! Manifest checks validate associations, not FD membership or application support.
//! Input and logical content/count budgets bound work, not exact peak allocator
//! usage. Encoding also caps its archive writer; wire conversion and serialization
//! scratch still need memory. Applications should choose limits for their trust boundary.
#![forbid(unsafe_code)]
mod wire;

use handle_off_bundle::{
    Bundle, BundleId, GenerationId, Manifest, ManifestError, Member, MemberId,
};
use handle_off_handles::ResourceId;
use handle_off_state::{Extension, Payload, StateError, StateFormat};
use std::{collections::BTreeSet, error::Error, fmt};

pub const WIRE_VERSION: u32 = 1;
const MAGIC: &[u8; 8] = b"HORKYV\0\0";
const HEADER: usize = 24;
/// Upper API ceiling; callers can impose lower bounds.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_NODES: usize = 1_000_000;
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Entire envelope, including header. Also bounds encoding output.
    pub max_bytes: usize,
    /// Resource, bundle, member and extension count, checked before deserialization.
    pub max_nodes: usize,
    /// Total logical UTF-8/inline byte content, checked before deserialization.
    pub max_content_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_nodes: 65_536,
            max_content_bytes: 16 * 1024 * 1024,
        }
    }
}
#[derive(Debug)]
pub enum CodecError {
    InvalidLimits,
    LimitExceeded,
    InvalidEnvelope,
    UnsupportedVersion(u32),
    WrongImageKind,
    InvalidResourceId,
    Archive(rkyv::rancor::Error),
    Manifest(ManifestError),
    State(StateError),
    DuplicateExtension(String),
}
impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "state codec error: {self:?}")
    }
}
impl Error for CodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Archive(error) => Some(error),
            Self::Manifest(error) => Some(error),
            Self::State(error) => Some(error),
            _ => None,
        }
    }
}
struct Budget {
    nodes: usize,
    bytes: usize,
}
impl Budget {
    fn new(limits: Limits) -> Result<Self, CodecError> {
        if limits.max_bytes < HEADER
            || limits.max_bytes > MAX_BYTES
            || limits.max_content_bytes > MAX_BYTES
            || limits.max_nodes > MAX_NODES
        {
            return Err(CodecError::InvalidLimits);
        }
        Ok(Self {
            nodes: limits.max_nodes,
            bytes: limits.max_content_bytes,
        })
    }
    fn node(&mut self) -> Result<(), CodecError> {
        self.nodes = self.nodes.checked_sub(1).ok_or(CodecError::LimitExceeded)?;
        Ok(())
    }
    fn bytes(&mut self, length: usize) -> Result<(), CodecError> {
        self.bytes = self
            .bytes
            .checked_sub(length)
            .ok_or(CodecError::LimitExceeded)?;
        Ok(())
    }
}
fn check_extensions(extensions: &[Extension]) -> Result<(), CodecError> {
    let mut keys = BTreeSet::new();
    for extension in extensions {
        extension.validate().map_err(CodecError::State)?;
        if !keys.insert(&extension.key) {
            return Err(CodecError::DuplicateExtension(extension.key.clone()));
        }
    }
    Ok(())
}
fn extension_to_wire(
    extension: &Extension,
    budget: &mut Budget,
) -> Result<wire::Extension, CodecError> {
    budget.node()?;
    budget.bytes(extension.key.len())?;
    budget.bytes(extension.format.name.len())?;
    let payload = match &extension.payload {
        Payload::Inline(bytes) => {
            budget.bytes(bytes.len())?;
            wire::Payload::Inline(bytes.clone())
        }
        Payload::ResourceRange {
            resource,
            offset,
            length,
        } => {
            budget.bytes(resource.as_str().len())?;
            wire::Payload::ResourceRange {
                resource: resource.as_str().into(),
                offset: *offset,
                length: *length,
            }
        }
    };
    Ok(wire::Extension {
        key: extension.key.clone(),
        format: extension.format.name.clone(),
        version: extension.format.version,
        required: extension.required,
        payload,
    })
}
fn extensions_to_wire(
    list: &[Extension],
    budget: &mut Budget,
) -> Result<Vec<wire::Extension>, CodecError> {
    list.iter()
        .map(|extension| extension_to_wire(extension, budget))
        .collect()
}
fn encode(image: wire::Image, limits: Limits) -> Result<Vec<u8>, CodecError> {
    struct Bounded {
        bytes: Vec<u8>,
        limit: usize,
        exceeded: bool,
    }
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit - self.bytes.len() {
                self.exceeded = true;
                return Err(std::io::Error::other("archive byte limit exceeded"));
            }
            self.bytes
                .try_reserve_exact(bytes.len())
                .map_err(std::io::Error::other)?;
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Bounded {
        bytes: Vec::new(),
        limit: limits.max_bytes - HEADER,
        exceeded: false,
    };
    let result = rkyv::api::high::to_bytes_in::<_, rkyv::rancor::Error>(
        &image,
        rkyv::ser::writer::IoWriter::new(&mut output),
    )
    .map(|_| ());
    if let Err(error) = result {
        return Err(if output.exceeded {
            CodecError::LimitExceeded
        } else {
            CodecError::Archive(error)
        });
    }
    let archive = output.bytes;
    let size = archive
        .len()
        .checked_add(HEADER)
        .ok_or(CodecError::LimitExceeded)?;
    if size > limits.max_bytes {
        return Err(CodecError::LimitExceeded);
    }
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&WIRE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes()); // reserved format flags
    bytes.extend_from_slice(&(archive.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&archive);
    Ok(bytes)
}
pub fn encode_manifest(manifest: &Manifest, limits: Limits) -> Result<Vec<u8>, CodecError> {
    let mut budget = Budget::new(limits)?;
    budget.bytes(manifest.generation.as_str().len())?;
    let resources = manifest
        .resources
        .iter()
        .map(|id| {
            budget.node()?;
            budget.bytes(id.as_str().len())?;
            Ok(id.as_str().to_owned())
        })
        .collect::<Result<_, CodecError>>()?;
    let mut bundles = Vec::new();
    for bundle in &manifest.bundles {
        budget.node()?;
        budget.bytes(bundle.id.as_str().len())?;
        let mut members = Vec::new();
        for member in &bundle.members {
            budget.node()?;
            budget.bytes(member.id.as_str().len())?;
            budget.bytes(member.resource.as_str().len())?;
            members.push(wire::Member {
                id: member.id.as_str().into(),
                resource: member.resource.as_str().into(),
                extensions: extensions_to_wire(&member.extensions, &mut budget)?,
            });
        }
        bundles.push(wire::Bundle {
            id: bundle.id.as_str().into(),
            members,
            extensions: extensions_to_wire(&bundle.extensions, &mut budget)?,
        });
    }
    let image = wire::Manifest {
        version: manifest.version,
        generation: manifest.generation.as_str().into(),
        resources,
        bundles,
        extensions: extensions_to_wire(&manifest.extensions, &mut budget)?,
    };
    manifest.validate().map_err(CodecError::Manifest)?;
    encode(wire::Image::Manifest(image), limits)
}
/// Independently encode opaque state metadata without requiring a bundle.
pub fn encode_extensions(extensions: &[Extension], limits: Limits) -> Result<Vec<u8>, CodecError> {
    let mut budget = Budget::new(limits)?;
    let image = wire::Image::Extensions(extensions_to_wire(extensions, &mut budget)?);
    check_extensions(extensions)?;
    encode(image, limits)
}
fn check_archived_extensions(
    list: &[wire::ArchivedExtension],
    budget: &mut Budget,
) -> Result<(), CodecError> {
    for extension in list {
        budget.node()?;
        budget.bytes(extension.key.len())?;
        budget.bytes(extension.format.len())?;
        match &extension.payload {
            wire::ArchivedPayload::Inline(bytes) => budget.bytes(bytes.len())?,
            wire::ArchivedPayload::ResourceRange { resource, .. } => {
                budget.bytes(resource.len())?
            }
        }
    }
    Ok(())
}
fn decode(bytes: &[u8], limits: Limits) -> Result<wire::Image, CodecError> {
    let mut budget = Budget::new(limits)?;
    if bytes.len() > limits.max_bytes {
        return Err(CodecError::LimitExceeded);
    }
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err(CodecError::InvalidEnvelope);
    }
    let version = u32::from_le_bytes(
        bytes[8..12]
            .try_into()
            .map_err(|_| CodecError::InvalidEnvelope)?,
    );
    if version != WIRE_VERSION {
        return Err(CodecError::UnsupportedVersion(version));
    }
    let length = u64::from_le_bytes(
        bytes[16..24]
            .try_into()
            .map_err(|_| CodecError::InvalidEnvelope)?,
    );
    if bytes[12..16] != [0; 4] || length != (bytes.len() - HEADER) as u64 {
        return Err(CodecError::InvalidEnvelope);
    }
    let archive = rkyv::access::<wire::ArchivedImage, rkyv::rancor::Error>(&bytes[HEADER..])
        .map_err(CodecError::Archive)?;
    // Traverse validated archived data before allocating the owned representation.
    match archive {
        wire::ArchivedImage::Extensions(list) => check_archived_extensions(list, &mut budget)?,
        wire::ArchivedImage::Manifest(manifest) => {
            budget.bytes(manifest.generation.len())?;
            for resource in manifest.resources.iter() {
                budget.node()?;
                budget.bytes(resource.len())?;
            }
            for bundle in manifest.bundles.iter() {
                budget.node()?;
                budget.bytes(bundle.id.len())?;
                for member in bundle.members.iter() {
                    budget.node()?;
                    budget.bytes(member.id.len())?;
                    budget.bytes(member.resource.len())?;
                    check_archived_extensions(&member.extensions, &mut budget)?;
                }
                check_archived_extensions(&bundle.extensions, &mut budget)?;
            }
            check_archived_extensions(&manifest.extensions, &mut budget)?;
        }
    }
    rkyv::deserialize::<wire::Image, rkyv::rancor::Error>(archive).map_err(CodecError::Archive)
}
fn resource(value: String) -> Result<ResourceId, CodecError> {
    ResourceId::new(value).map_err(|_| CodecError::InvalidResourceId)
}
fn extension_from_wire(extension: wire::Extension) -> Result<Extension, CodecError> {
    let payload = match extension.payload {
        wire::Payload::Inline(bytes) => Payload::Inline(bytes),
        wire::Payload::ResourceRange {
            resource: id,
            offset,
            length,
        } => Payload::ResourceRange {
            resource: resource(id)?,
            offset,
            length,
        },
    };
    Ok(Extension {
        key: extension.key,
        format: StateFormat {
            name: extension.format,
            version: extension.version,
        },
        required: extension.required,
        payload,
    })
}
fn extensions_from_wire(list: Vec<wire::Extension>) -> Result<Vec<Extension>, CodecError> {
    list.into_iter().map(extension_from_wire).collect()
}
pub fn decode_extensions(bytes: &[u8], limits: Limits) -> Result<Vec<Extension>, CodecError> {
    let wire::Image::Extensions(list) = decode(bytes, limits)? else {
        return Err(CodecError::WrongImageKind);
    };
    let extensions = extensions_from_wire(list)?;
    check_extensions(&extensions)?;
    Ok(extensions)
}
pub fn decode_manifest(bytes: &[u8], limits: Limits) -> Result<Manifest, CodecError> {
    let wire::Image::Manifest(image) = decode(bytes, limits)? else {
        return Err(CodecError::WrongImageKind);
    };
    let resources = image
        .resources
        .into_iter()
        .map(resource)
        .collect::<Result<_, _>>()?;
    let mut bundles = Vec::new();
    for bundle in image.bundles {
        let mut members = Vec::new();
        for member in bundle.members {
            members.push(Member {
                id: MemberId::new(member.id).map_err(CodecError::Manifest)?,
                resource: resource(member.resource)?,
                extensions: extensions_from_wire(member.extensions)?,
            });
        }
        bundles.push(Bundle {
            id: BundleId::new(bundle.id).map_err(CodecError::Manifest)?,
            members,
            extensions: extensions_from_wire(bundle.extensions)?,
        });
    }
    let manifest = Manifest {
        version: image.version,
        generation: GenerationId::new(image.generation).map_err(CodecError::Manifest)?,
        resources,
        bundles,
        extensions: extensions_from_wire(image.extensions)?,
    };
    manifest.validate().map_err(CodecError::Manifest)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn extensions() -> Vec<Extension> {
        vec![Extension {
            key: "opaque".into(),
            format: StateFormat {
                name: "application.state".into(),
                version: 999,
            },
            required: true,
            payload: Payload::Inline(vec![0, 0xff, 1]),
        }]
    }
    fn manifest() -> Manifest {
        let output = ResourceId::new("output").unwrap();
        Manifest {
            version: handle_off_bundle::MANIFEST_VERSION,
            generation: GenerationId::new("generation-1").unwrap(),
            resources: vec![output.clone()],
            bundles: vec![Bundle {
                id: BundleId::new("stdio").unwrap(),
                members: vec![Member {
                    id: MemberId::new("stdout").unwrap(),
                    resource: output,
                    extensions: extensions(),
                }],
                extensions: Vec::new(),
            }],
            extensions: extensions(),
        }
    }
    fn wire_manifest() -> wire::Manifest {
        wire::Manifest {
            version: 1,
            generation: "g1".into(),
            resources: vec!["output".into()],
            bundles: vec![wire::Bundle {
                id: "bundle".into(),
                members: vec![wire::Member {
                    id: "member".into(),
                    resource: "output".into(),
                    extensions: Vec::new(),
                }],
                extensions: Vec::new(),
            }],
            extensions: Vec::new(),
        }
    }
    #[test]
    fn roundtrips_preserve_opaque_state_and_support_unaligned_input() {
        let manifest = manifest();
        let bytes = encode_manifest(&manifest, Limits::default()).unwrap();
        let mut unaligned = vec![0];
        unaligned.extend_from_slice(&bytes);
        assert_eq!(
            decode_manifest(&unaligned[1..], Limits::default()).unwrap(),
            manifest
        );
        let extensions = extensions();
        let bytes = encode_extensions(&extensions, Limits::default()).unwrap();
        assert_eq!(
            decode_extensions(&bytes, Limits::default()).unwrap(),
            extensions
        );
        assert!(matches!(
            decode_manifest(&bytes, Limits::default()),
            Err(CodecError::WrongImageKind)
        ));
    }
    #[test]
    fn malformed_truncated_and_versioned_envelopes_return_errors() {
        let bytes = encode_manifest(&manifest(), Limits::default()).unwrap();
        for length in 0..bytes.len() {
            assert!(decode_manifest(&bytes[..length], Limits::default()).is_err());
        }
        let mut unsupported = bytes.clone();
        unsupported[8..12].copy_from_slice(&999u32.to_le_bytes());
        assert!(matches!(
            decode_manifest(&unsupported, Limits::default()),
            Err(CodecError::UnsupportedVersion(999))
        ));
        let mut reserved = bytes.clone();
        reserved[12] = 1;
        assert!(matches!(
            decode_manifest(&reserved, Limits::default()),
            Err(CodecError::InvalidEnvelope)
        ));
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(matches!(
            decode_manifest(&trailing, Limits::default()),
            Err(CodecError::InvalidEnvelope)
        ));
        let mut corrupt = bytes;
        corrupt[HEADER..].fill(0xff);
        assert!(matches!(
            decode_manifest(&corrupt, Limits::default()),
            Err(CodecError::Archive(_))
        ));
    }
    #[test]
    fn byte_valid_archives_still_require_semantic_validation() {
        let mut manifest = wire_manifest();
        manifest.resources.push("output".into());
        let bytes = encode(wire::Image::Manifest(manifest), Limits::default()).unwrap();
        assert!(matches!(
            decode_manifest(&bytes, Limits::default()),
            Err(CodecError::Manifest(ManifestError::DuplicateResource(_)))
        ));
        let mut manifest = wire_manifest();
        manifest.version = 999;
        let bytes = encode(wire::Image::Manifest(manifest), Limits::default()).unwrap();
        assert!(matches!(
            decode_manifest(&bytes, Limits::default()),
            Err(CodecError::Manifest(ManifestError::UnsupportedVersion(999)))
        ));
        let mut manifest = wire_manifest();
        manifest.bundles[0].members[0].resource = "absent".into();
        let bytes = encode(wire::Image::Manifest(manifest), Limits::default()).unwrap();
        assert!(matches!(
            decode_manifest(&bytes, Limits::default()),
            Err(CodecError::Manifest(ManifestError::MissingResource(_)))
        ));
        let mut manifest = wire_manifest();
        manifest.resources[0] = "bad\0id".into();
        let bytes = encode(wire::Image::Manifest(manifest), Limits::default()).unwrap();
        assert!(matches!(
            decode_manifest(&bytes, Limits::default()),
            Err(CodecError::InvalidResourceId)
        ));
        let extensions = vec![wire::Extension {
            key: "state".into(),
            format: "app.state".into(),
            version: 1,
            required: true,
            payload: wire::Payload::ResourceRange {
                resource: "image".into(),
                offset: u64::MAX,
                length: 1,
            },
        }];
        let bytes = encode(wire::Image::Extensions(extensions), Limits::default()).unwrap();
        assert!(matches!(
            decode_extensions(&bytes, Limits::default()),
            Err(CodecError::State(StateError::RangeOverflow))
        ));
    }
    #[test]
    fn limits_bound_input_and_owned_content_before_deserialization() {
        let bytes = encode_manifest(&manifest(), Limits::default()).unwrap();
        let too_small = Limits {
            max_bytes: bytes.len() - 1,
            ..Limits::default()
        };
        assert!(matches!(
            decode_manifest(&bytes, too_small),
            Err(CodecError::LimitExceeded)
        ));
        assert!(matches!(
            encode_manifest(&manifest(), too_small),
            Err(CodecError::LimitExceeded)
        ));
        for limits in [
            Limits {
                max_nodes: 0,
                ..Limits::default()
            },
            Limits {
                max_content_bytes: 0,
                ..Limits::default()
            },
        ] {
            assert!(matches!(
                decode_manifest(&bytes, limits),
                Err(CodecError::LimitExceeded)
            ));
            assert!(matches!(
                encode_manifest(&manifest(), limits),
                Err(CodecError::LimitExceeded)
            ));
        }
        assert!(matches!(
            decode_manifest(
                &bytes,
                Limits {
                    max_bytes: MAX_BYTES + 1,
                    ..Limits::default()
                }
            ),
            Err(CodecError::InvalidLimits)
        ));
        let mut invalid = extensions();
        invalid.push(invalid[0].clone());
        assert!(matches!(
            encode_extensions(&invalid, Limits::default()),
            Err(CodecError::DuplicateExtension(_))
        ));
    }
    #[test]
    fn arbitrary_bounded_bytes_never_need_unchecked_access() {
        // Deterministic garbage, plus single-byte mutations of a valid archive.
        // A mutation may produce valid metadata: this is not an integrity check.
        let seed = encode_manifest(&manifest(), Limits::default()).unwrap();
        for index in HEADER..seed.len() {
            let mut bytes = seed.clone();
            bytes[index] ^= 0xff;
            let _ = decode_manifest(&bytes, Limits::default());
        }
        for length in HEADER..128 {
            let mut bytes = vec![0xa5; length];
            bytes[..8].copy_from_slice(MAGIC);
            bytes[8..12].copy_from_slice(&WIRE_VERSION.to_le_bytes());
            bytes[12..16].fill(0);
            bytes[16..24].copy_from_slice(&((length - HEADER) as u64).to_le_bytes());
            assert!(decode_manifest(&bytes, Limits::default()).is_err());
        }
    }
}
