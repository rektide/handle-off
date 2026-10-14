//! Logical resource references, not descriptor numbers or kernel identity proofs.
use std::{error::Error, fmt};

/// A caller-assigned identity within one resource inventory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidResourceId;
impl fmt::Display for InvalidResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("resource ID must be nonempty and contain no control characters")
    }
}
impl Error for InvalidResourceId {}
impl ResourceId {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidResourceId> {
        let value = value.into();
        if value.is_empty() || value.chars().any(char::is_control) {
            return Err(InvalidResourceId);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(unix)]
mod unix {
    use super::ResourceId;
    use std::{
        collections::BTreeMap,
        os::fd::{AsFd, BorrowedFd, OwnedFd},
    };

    /// One reference per declared resource. Repeated roles share its ID.
    /// Distinct IDs may still refer to the same open file description; this table
    /// neither detects nor prohibits that. Dropping a descriptor never shuts it down.
    #[derive(Debug)]
    pub struct HandleTable<H> {
        entries: BTreeMap<ResourceId, H>,
    }
    pub type OwnedHandles = HandleTable<OwnedFd>;
    pub type BorrowedHandles<'a> = HandleTable<BorrowedFd<'a>>;
    impl<H> Default for HandleTable<H> {
        fn default() -> Self {
            Self {
                entries: BTreeMap::new(),
            }
        }
    }
    impl<H: AsFd> HandleTable<H> {
        /// On collision return the supplied handle without replacing the old one.
        pub fn insert(&mut self, id: ResourceId, handle: H) -> Result<(), H> {
            match self.entries.entry(id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(handle);
                    Ok(())
                }
                std::collections::btree_map::Entry::Occupied(_) => Err(handle),
            }
        }
        pub fn get(&self, id: &ResourceId) -> Option<&H> {
            self.entries.get(id)
        }
        pub fn remove(&mut self, id: &ResourceId) -> Option<H> {
            self.entries.remove(id)
        }
        pub fn iter(&self) -> impl ExactSizeIterator<Item = (&ResourceId, &H)> {
            self.entries.iter()
        }
        pub fn len(&self) -> usize {
            self.entries.len()
        }
        pub fn is_empty(&self) -> bool {
            self.entries.is_empty()
        }
        pub fn borrow(&self) -> BorrowedHandles<'_> {
            HandleTable {
                entries: self
                    .entries
                    .iter()
                    .map(|(id, fd)| (id.clone(), fd.as_fd()))
                    .collect(),
            }
        }
        /// Explicit duplication shares offsets/status flags, not application I/O permission.
        pub fn try_clone_to_owned(&self) -> std::io::Result<OwnedHandles> {
            Ok(HandleTable {
                entries: self
                    .entries
                    .iter()
                    .map(|(id, fd)| Ok((id.clone(), fd.as_fd().try_clone_to_owned()?)))
                    .collect::<std::io::Result<_>>()?,
            })
        }
    }
}
#[cfg(unix)]
pub use unix::{BorrowedHandles, HandleTable, OwnedHandles};

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        os::unix::net::UnixStream,
    };
    #[test]
    fn collision_preserves_both_owners_and_duplicates_keep_socket_alive() {
        let (socket, mut peer) = UnixStream::pair().unwrap();
        let (other, _peer) = UnixStream::pair().unwrap();
        let id = ResourceId::new("connection").unwrap();
        let mut table = OwnedHandles::default();
        table.insert(id.clone(), socket.into()).unwrap();
        let rejected = table.insert(id.clone(), other.into()).unwrap_err();
        drop(rejected);
        let copy = table.borrow().try_clone_to_owned().unwrap();
        drop(table);
        let mut stream = UnixStream::from(copy.into_fd_for_test(&id));
        peer.write_all(b"still open").unwrap();
        let mut bytes = [0; 10];
        stream.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"still open");
        drop(stream);
        assert_eq!(peer.read(&mut bytes).unwrap(), 0);
    }
    impl OwnedHandles {
        fn into_fd_for_test(mut self, id: &ResourceId) -> std::os::fd::OwnedFd {
            self.remove(id).unwrap()
        }
    }
    #[test]
    fn invalid_ids() {
        for value in ["", "a\n", "a\0"] {
            assert!(ResourceId::new(value).is_err());
        }
    }
}
