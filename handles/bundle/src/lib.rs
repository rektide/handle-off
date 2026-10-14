//! Associations only: transport positions and manager names are adapter bindings.
//! Metadata validation does not authorize I/O or establish kernel resource identity.
use handle_off_handles::ResourceId;
use handle_off_state::{Extension, Payload, StateError};
use std::{collections::BTreeSet, error::Error, fmt};

macro_rules! identity {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ManifestError> {
                let value = value.into();
                if value.is_empty() || value.chars().any(char::is_control) {
                    return Err(ManifestError::InvalidIdentity);
                }
                Ok(Self(value))
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}
identity!(GenerationId);
identity!(BundleId);
identity!(MemberId);
/// Semantic schema version; no concrete serialized wire envelope is defined yet.
pub const MANIFEST_VERSION: u32 = 1;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub id: MemberId,
    pub resource: ResourceId,
    pub extensions: Vec<Extension>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    pub id: BundleId,
    pub members: Vec<Member>,
    pub extensions: Vec<Extension>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub version: u32,
    pub generation: GenerationId,
    pub resources: Vec<ResourceId>,
    pub bundles: Vec<Bundle>,
    pub extensions: Vec<Extension>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestError {
    InvalidIdentity,
    UnsupportedVersion(u32),
    DuplicateResource(ResourceId),
    DuplicateBundle(BundleId),
    DuplicateMember { bundle: BundleId, member: MemberId },
    DuplicateExtension(String),
    MissingResource(ResourceId),
    UnreferencedResource(ResourceId),
    HandleMembership,
    State(StateError),
}
impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "manifest validation failed: {self:?}")
    }
}
impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::State(error) = self {
            Some(error)
        } else {
            None
        }
    }
}
impl Manifest {
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.version != MANIFEST_VERSION {
            return Err(ManifestError::UnsupportedVersion(self.version));
        }
        let mut resources = BTreeSet::new();
        for resource in &self.resources {
            if !resources.insert(resource) {
                return Err(ManifestError::DuplicateResource(resource.clone()));
            }
        }
        let mut referenced = BTreeSet::new();
        let mut reference = |resource: &ResourceId| -> Result<(), ManifestError> {
            if !resources.contains(resource) {
                return Err(ManifestError::MissingResource(resource.clone()));
            }
            referenced.insert(resource.clone());
            Ok(())
        };
        fn extensions(
            list: &[Extension],
            reference: &mut impl FnMut(&ResourceId) -> Result<(), ManifestError>,
        ) -> Result<(), ManifestError> {
            let mut keys = BTreeSet::new();
            for extension in list {
                extension.validate().map_err(ManifestError::State)?;
                if !keys.insert(&extension.key) {
                    return Err(ManifestError::DuplicateExtension(extension.key.clone()));
                }
                if let Payload::ResourceRange { resource, .. } = &extension.payload {
                    reference(resource)?;
                }
            }
            Ok(())
        }
        extensions(&self.extensions, &mut reference)?;
        let mut bundles = BTreeSet::new();
        for bundle in &self.bundles {
            if !bundles.insert(&bundle.id) {
                return Err(ManifestError::DuplicateBundle(bundle.id.clone()));
            }
            extensions(&bundle.extensions, &mut reference)?;
            let mut members = BTreeSet::new();
            for member in &bundle.members {
                if !members.insert(&member.id) {
                    return Err(ManifestError::DuplicateMember {
                        bundle: bundle.id.clone(),
                        member: member.id.clone(),
                    });
                }
                reference(&member.resource)?;
                extensions(&member.extensions, &mut reference)?;
            }
        }
        for resource in resources {
            if !referenced.contains(resource) {
                return Err(ManifestError::UnreferencedResource(resource.clone()));
            }
        }
        Ok(())
    }
    /// Require the complete, exact declared inventory, independent of delivery order.
    #[cfg(unix)]
    pub fn validate_handles<H: std::os::fd::AsFd>(
        &self,
        handles: &handle_off_handles::HandleTable<H>,
    ) -> Result<(), ManifestError> {
        self.validate()?;
        if handles.len() != self.resources.len()
            || self.resources.iter().any(|id| handles.get(id).is_none())
        {
            return Err(ManifestError::HandleMembership);
        }
        Ok(())
    }
}
