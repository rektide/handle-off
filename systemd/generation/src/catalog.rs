use crate::{Address, GenerationKey, Member, NameError, NamespaceToken, ResourceSlot};
use handle_off_bundle::{GenerationId, Manifest, ManifestError};
use std::{collections::BTreeMap, error::Error, fmt};
pub const MAX_ENTRIES: usize = 65_536;
pub const MAX_METADATA_NODES: usize = 65_536;
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_entries: usize,
    pub max_generations: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 4096,
            max_generations: 256,
        }
    }
}
/// Borrowed opaque handle/data; validity and ownership remain with the caller.
/// Can be built from activation entries without another raw-FD claiming boundary.
pub struct NamedHandle<'a, H> {
    pub name: Option<&'a str>,
    pub handle: &'a H,
}
#[derive(Debug)]
pub struct HandleRef<'a, H> {
    pub input_index: usize,
    pub handle: &'a H,
}
#[derive(Debug)]
pub struct Unrelated<'a, H> {
    pub input_index: usize,
    pub name: Option<&'a str>,
    pub handle: &'a H,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogError {
    InvalidLimits,
    EntryLimit,
    GenerationLimit,
    MalformedName {
        input_index: usize,
        cause: NameError,
    },
    Duplicate {
        address: Address,
        first_input: usize,
        second_input: usize,
    },
}
impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "generation catalog error: {self:?}")
    }
}
impl Error for CatalogError {}
#[derive(Debug)]
pub struct Candidate<'a, H> {
    key: GenerationKey,
    root: Option<HandleRef<'a, H>>,
    marker: Option<HandleRef<'a, H>>,
    resources: BTreeMap<ResourceSlot, HandleRef<'a, H>>,
}
#[derive(Debug)]
pub struct Catalog<'a, H> {
    candidates: BTreeMap<GenerationKey, Candidate<'a, H>>,
    unrelated: Vec<Unrelated<'a, H>>,
}
impl<'a, H: 'a> Catalog<'a, H> {
    /// Limits are enforced during iteration, before allocating another candidate.
    /// Failure drops only borrowed references: caller inventory is never consumed.
    /// BTree ordering is deterministic presentation, NOT time/freshness precedence.
    pub fn discover(
        inputs: impl IntoIterator<Item = NamedHandle<'a, H>>,
        limits: Limits,
    ) -> Result<Self, CatalogError> {
        if limits.max_entries > MAX_ENTRIES || limits.max_generations > MAX_ENTRIES {
            return Err(CatalogError::InvalidLimits);
        }
        let mut catalog = Self {
            candidates: BTreeMap::new(),
            unrelated: Vec::new(),
        };
        for (input_index, input) in inputs.into_iter().enumerate() {
            if input_index >= limits.max_entries {
                return Err(CatalogError::EntryLimit);
            }
            let address = input
                .name
                .map(Address::parse)
                .transpose()
                .map_err(|cause| CatalogError::MalformedName { input_index, cause })?
                .flatten();
            let Some(address) = address else {
                catalog.unrelated.push(Unrelated {
                    input_index,
                    name: input.name,
                    handle: input.handle,
                });
                continue;
            };
            if !catalog.candidates.contains_key(&address.key)
                && catalog.candidates.len() >= limits.max_generations
            {
                return Err(CatalogError::GenerationLimit);
            }
            let candidate = catalog
                .candidates
                .entry(address.key)
                .or_insert_with(|| Candidate {
                    key: address.key,
                    root: None,
                    marker: None,
                    resources: BTreeMap::new(),
                });
            let reference = HandleRef {
                input_index,
                handle: input.handle,
            };
            let previous = match address.member {
                Member::Root => candidate.root.as_ref(),
                Member::CommitMarker => candidate.marker.as_ref(),
                Member::Resource(slot) => candidate.resources.get(&slot),
            };
            if let Some(previous) = previous {
                return Err(CatalogError::Duplicate {
                    address,
                    first_input: previous.input_index,
                    second_input: input_index,
                });
            }
            match address.member {
                Member::Root => candidate.root = Some(reference),
                Member::CommitMarker => candidate.marker = Some(reference),
                Member::Resource(slot) => {
                    candidate.resources.insert(slot, reference);
                }
            }
        }
        Ok(catalog)
    }
    pub fn candidates(&self) -> &BTreeMap<GenerationKey, Candidate<'a, H>> {
        &self.candidates
    }
    pub fn candidate(&self, key: GenerationKey) -> Option<&Candidate<'a, H>> {
        self.candidates.get(&key)
    }
    pub fn namespace(&self, namespace: NamespaceToken) -> impl Iterator<Item = &Candidate<'a, H>> {
        self.candidates
            .values()
            .filter(move |candidate| candidate.key.namespace == namespace)
    }
    pub fn unrelated(&self) -> &[Unrelated<'a, H>] {
        &self.unrelated
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MappingError {
    MissingRoot,
    MissingMarker,
    ResourceLimit,
    MetadataLimit,
    Manifest(ManifestError),
    ResourceCount { expected: usize, actual: usize },
    MissingSlot(ResourceSlot),
}
impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "generation mapping error: {self:?}")
    }
}
impl Error for MappingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        if let Self::Manifest(error) = self {
            Some(error)
        } else {
            None
        }
    }
}
// Mapping borrows logical IDs; arbitrary application strings never become FDNAMEs.
#[derive(Debug)]
pub struct ResourceBinding<'m, 'h, H> {
    pub slot: ResourceSlot,
    pub resource_id: &'m handle_off_handles::ResourceId,
    pub handle: &'h H,
}
#[derive(Debug)]
pub struct ManifestMapping<'m, 'h, H> {
    pub key: GenerationKey,
    pub logical_generation: &'m GenerationId,
    pub root: &'h H,
    pub commit_marker: Option<&'h H>,
    pub resources: Vec<ResourceBinding<'m, 'h, H>>,
}
impl<'a, H> Candidate<'a, H> {
    pub fn key(&self) -> GenerationKey {
        self.key
    }
    pub fn root(&self) -> Option<&HandleRef<'a, H>> {
        self.root.as_ref()
    }
    pub fn commit_marker(&self) -> Option<&HandleRef<'a, H>> {
        self.marker.as_ref()
    }
    pub fn resources(&self) -> &BTreeMap<ResourceSlot, HandleRef<'a, H>> {
        &self.resources
    }
    /// Physical presence only: this does not validate the marker's content or root
    /// linkage, much less authorize execution. Prepared generations may lack it.
    pub fn require_marker(&self) -> Result<&'a H, MappingError> {
        self.marker
            .as_ref()
            .map(|reference| reference.handle)
            .ok_or(MappingError::MissingMarker)
    }
    /// Validate semantic manifest and exact contiguous slot inventory. No marker is
    /// required for a PREPARED mapping, and this return value is never an execution
    /// approval. Caller must establish that the supplied decoded manifest actually
    /// belongs to this root; logical generation IDs are not backend token encodings.
    pub fn map_manifest<'m>(
        &self,
        manifest: &'m Manifest,
    ) -> Result<ManifestMapping<'m, 'a, H>, MappingError> {
        let root = self.root.as_ref().ok_or(MappingError::MissingRoot)?.handle;
        if manifest.resources.len() > MAX_ENTRIES {
            return Err(MappingError::ResourceLimit);
        }
        let mut nodes = manifest
            .resources
            .len()
            .checked_add(manifest.extensions.len())
            .ok_or(MappingError::MetadataLimit)?;
        for bundle in &manifest.bundles {
            nodes = nodes
                .checked_add(1)
                .and_then(|n| n.checked_add(bundle.members.len()))
                .and_then(|n| n.checked_add(bundle.extensions.len()))
                .ok_or(MappingError::MetadataLimit)?;
            if nodes > MAX_METADATA_NODES {
                return Err(MappingError::MetadataLimit);
            }
            for member in &bundle.members {
                nodes = nodes
                    .checked_add(member.extensions.len())
                    .ok_or(MappingError::MetadataLimit)?;
                if nodes > MAX_METADATA_NODES {
                    return Err(MappingError::MetadataLimit);
                }
            }
        }
        if nodes > MAX_METADATA_NODES {
            return Err(MappingError::MetadataLimit);
        }
        manifest.validate().map_err(MappingError::Manifest)?;
        if self.resources.len() != manifest.resources.len() {
            return Err(MappingError::ResourceCount {
                expected: manifest.resources.len(),
                actual: self.resources.len(),
            });
        }
        let mut resources = Vec::with_capacity(manifest.resources.len());
        for (index, resource) in manifest.resources.iter().enumerate() {
            let slot = ResourceSlot::new(index as u32); // resource bound above fits u32
            let reference = self
                .resources
                .get(&slot)
                .ok_or(MappingError::MissingSlot(slot))?;
            resources.push(ResourceBinding {
                slot,
                resource_id: resource,
                handle: reference.handle,
            });
        }
        Ok(ManifestMapping {
            key: self.key,
            logical_generation: &manifest.generation,
            root,
            commit_marker: self.marker.as_ref().map(|reference| reference.handle),
            resources,
        })
    }
}
