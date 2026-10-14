use handle_off_bundle::{GenerationId, Manifest, ManifestError};
use handle_off_handles::{HandleTable, OwnedHandles};
use handle_off_state_rkyv::{self as codec, CodecError};
use handle_off_unix_rights::{self as rights, ReceiveLimits};
use std::{
    error::Error as StdError,
    fmt, io,
    os::{fd::AsFd, unix::net::UnixDatagram},
};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_DATAGRAM: usize = 64 * 1024;
pub const MAX_GENERATION_BYTES: usize = 256;
const MAGIC: &[u8; 8] = b"HOUDG\0\0\0";
const HEADER: usize = 28;
const OFFER: u8 = 1;
const ACCEPTED: u8 = 2;
const REJECTED: u8 = 3;

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_datagram: usize,
    pub max_fds: usize,
    pub max_nodes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_datagram: 16 * 1024,
            max_fds: 64,
            max_nodes: 4096,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_datagram < HEADER + 24
            || self.max_datagram > MAX_DATAGRAM
            || self.max_fds > rights::MAX_FDS
            || self.max_nodes > codec::MAX_NODES
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn codec(self) -> codec::Limits {
        codec::Limits {
            max_bytes: self.max_datagram,
            max_content_bytes: self.max_datagram,
            max_nodes: self.max_nodes,
        }
    }
    fn receive(self) -> ReceiveLimits {
        ReceiveLimits {
            max_payload: self.max_datagram,
            max_fds: self.max_fds,
        }
    }
}
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Codec(CodecError),
    Manifest(ManifestError),
    InvalidLimits,
    Poisoned,
    InvalidFrame,
    UnsupportedVersion(u16),
    UnexpectedKind,
    DescriptorCountMismatch,
    GenerationMismatch,
    DuplicateOffer,
    UnrelatedResponse,
    LimitExceeded,
    IdExhausted,
    Rejected,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unix bundle protocol error: {self:?}")
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Codec(error) => Some(error),
            Self::Manifest(error) => Some(error),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendPhase {
    BeforeSend,
    Indeterminate,
    Rejected,
}
#[derive(Debug)]
pub struct SendError {
    pub phase: SendPhase,
    pub error: Error,
}
impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "delivery {:?}: {}", self.phase, self.error)
    }
}
impl StdError for SendError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
#[derive(Debug)]
pub struct Receipt {
    pub offer_id: u64,
    pub generation: GenerationId,
}
#[derive(Debug)]
pub struct Delivery {
    pub manifest: Manifest,
    pub handles: OwnedHandles,
}
#[derive(Debug)]
pub struct AcceptError {
    pub error: Error,
    /// Retained owners; unbox to recover the delivery after an ACK failure.
    pub delivery: Box<Delivery>,
}
impl fmt::Display for AcceptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "acceptance decision made but ACK failed: {}", self.error)
    }
}
impl StdError for AcceptError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}

struct Frame<'a> {
    kind: u8,
    id: u64,
    generation: &'a str,
    count: usize,
    body: &'a [u8],
}
fn frame(
    kind: u8,
    id: u64,
    generation: &str,
    count: usize,
    body: &[u8],
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    if generation.is_empty() || generation.len() > MAX_GENERATION_BYTES || count > limits.max_fds {
        return Err(Error::LimitExceeded);
    }
    let size = HEADER
        .checked_add(generation.len())
        .and_then(|size| size.checked_add(body.len()))
        .ok_or(Error::LimitExceeded)?;
    if size > limits.max_datagram {
        return Err(Error::LimitExceeded);
    }
    let mut bytes = Vec::with_capacity(size);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
    bytes.push(kind);
    bytes.push(0);
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&(generation.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&(count as u16).to_le_bytes());
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(generation.as_bytes());
    bytes.extend_from_slice(body);
    Ok(bytes)
}
fn parse(bytes: &[u8]) -> Result<Frame<'_>, Error> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err(Error::InvalidFrame);
    }
    let u16_at = |index| -> Result<u16, Error> {
        Ok(u16::from_le_bytes(
            bytes[index..index + 2]
                .try_into()
                .map_err(|_| Error::InvalidFrame)?,
        ))
    };
    let version = u16_at(8)?;
    if version != PROTOCOL_VERSION {
        return Err(Error::UnsupportedVersion(version));
    }
    if bytes[11] != 0 || !matches!(bytes[10], OFFER | ACCEPTED | REJECTED) {
        return Err(Error::InvalidFrame);
    }
    let id = u64::from_le_bytes(bytes[12..20].try_into().map_err(|_| Error::InvalidFrame)?);
    let generation_len = u16_at(20)? as usize;
    let count = u16_at(22)? as usize;
    let body_len =
        u32::from_le_bytes(bytes[24..28].try_into().map_err(|_| Error::InvalidFrame)?) as usize;
    let body_start = HEADER
        .checked_add(generation_len)
        .ok_or(Error::InvalidFrame)?;
    if id == 0
        || generation_len == 0
        || generation_len > MAX_GENERATION_BYTES
        || body_start.checked_add(body_len) != Some(bytes.len())
    {
        return Err(Error::InvalidFrame);
    }
    let generation =
        std::str::from_utf8(&bytes[HEADER..body_start]).map_err(|_| Error::InvalidFrame)?;
    if generation.chars().any(char::is_control) {
        return Err(Error::InvalidFrame);
    }
    if bytes[10] != OFFER && (count != 0 || body_len != 0) {
        return Err(Error::InvalidFrame);
    }
    Ok(Frame {
        kind: bytes[10],
        id,
        generation,
        count,
        body: &bytes[body_start..],
    })
}

#[derive(Debug)]
pub struct Endpoint {
    socket: UnixDatagram,
    limits: Limits,
    next_offer: u64,
    last_received: u64,
    poisoned: bool,
}
impl Endpoint {
    /// Takes exclusive protocol custody of a fresh, connected socket. Do not retain
    /// clones or independently read/write the endpoint. Transport size ceilings do
    /// not guarantee kernel send-buffer capacity on every Unix platform.
    pub fn new(socket: UnixDatagram, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        socket.peer_addr()?;
        Ok(Self {
            socket,
            limits,
            next_offer: 1,
            last_received: 0,
            poisoned: false,
        })
    }
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    /// Planned correlation for the next offer under exclusive endpoint use. This
    /// does not submit it or promise success; useful for a separate control session
    /// whose abort must also correlate when a delivery ACK is lost.
    pub fn next_offer_id(&self) -> u64 {
        self.next_offer
    }
    /// No timeout is installed by the protocol. Configure the caller's socket
    /// before construction; a missing ACK on a blocking endpoint may wait forever.
    pub fn send<H: AsFd>(
        &mut self,
        manifest: &Manifest,
        handles: &HandleTable<H>,
    ) -> Result<Receipt, SendError> {
        let before = |error| SendError {
            phase: SendPhase::BeforeSend,
            error,
        };
        if self.poisoned {
            return Err(before(Error::Poisoned));
        }
        manifest
            .validate_handles(handles)
            .map_err(|error| before(Error::Manifest(error)))?;
        if manifest.resources.len() > self.limits.max_fds {
            return Err(before(Error::LimitExceeded));
        }
        let next = self
            .next_offer
            .checked_add(1)
            .ok_or_else(|| before(Error::IdExhausted))?;
        let body = codec::encode_manifest(manifest, self.limits.codec())
            .map_err(|error| before(Error::Codec(error)))?;
        let bytes = frame(
            OFFER,
            self.next_offer,
            manifest.generation.as_str(),
            manifest.resources.len(),
            &body,
            self.limits,
        )
        .map_err(before)?;
        let mut descriptors = Vec::with_capacity(manifest.resources.len());
        for id in &manifest.resources {
            let handle = handles
                .get(id)
                .ok_or_else(|| before(Error::Manifest(ManifestError::HandleMembership)))?;
            descriptors.push(handle.as_fd());
        }
        let id = self.next_offer;
        self.next_offer = next;
        self.poisoned = true;
        let uncertain = |error| SendError {
            phase: SendPhase::Indeterminate,
            error,
        };
        rights::send(&self.socket, &bytes, &descriptors)
            .map_err(|error| uncertain(Error::Io(error)))?;
        let response = rights::receive(
            &self.socket,
            ReceiveLimits {
                max_payload: self.limits.max_datagram,
                max_fds: 0,
            },
        )
        .map_err(|error| uncertain(Error::Io(error)))?;
        let response = parse(&response.payload).map_err(uncertain)?;
        if response.id != id || response.generation != manifest.generation.as_str() {
            return Err(uncertain(Error::UnrelatedResponse));
        }
        match response.kind {
            ACCEPTED => {
                self.poisoned = false;
                Ok(Receipt {
                    offer_id: id,
                    generation: manifest.generation.clone(),
                })
            }
            REJECTED => {
                self.poisoned = false;
                Err(SendError {
                    phase: SendPhase::Rejected,
                    error: Error::Rejected,
                })
            }
            _ => Err(uncertain(Error::UnexpectedKind)),
        }
    }
    /// Validate one offer's framing, manifest semantics and exact FD inventory.
    /// No ACK is emitted. Application validation precedes explicit accept/reject.
    /// On malformed input all acquired owners are dropped and the endpoint poisons.
    pub fn receive(&mut self) -> Result<Offer<'_>, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        self.poisoned = true;
        let message = rights::receive(&self.socket, self.limits.receive())?;
        let packet = parse(&message.payload)?;
        if packet.kind != OFFER {
            return Err(Error::UnexpectedKind);
        }
        if packet.id <= self.last_received {
            return Err(Error::DuplicateOffer);
        }
        if packet.count != message.descriptors.len() {
            return Err(Error::DescriptorCountMismatch);
        }
        let manifest =
            codec::decode_manifest(packet.body, self.limits.codec()).map_err(Error::Codec)?;
        if manifest.generation.as_str() != packet.generation {
            return Err(Error::GenerationMismatch);
        }
        if manifest.resources.len() != message.descriptors.len() {
            return Err(Error::DescriptorCountMismatch);
        }
        let mut handles = OwnedHandles::default();
        for (id, fd) in manifest.resources.iter().cloned().zip(message.descriptors) {
            // Semantics already reject duplicate IDs; retain defensive ownership cleanup.
            handles
                .insert(id, fd)
                .map_err(|_| Error::DescriptorCountMismatch)?;
        }
        manifest
            .validate_handles(&handles)
            .map_err(Error::Manifest)?;
        self.last_received = packet.id;
        Ok(Offer {
            endpoint: self,
            id: packet.id,
            delivery: Delivery { manifest, handles },
        })
    }
}
/// Pending application decision. Borrowing excludes another operation on this
/// endpoint. Dropping closes acquired references without ACK and leaves it poisoned.
#[derive(Debug)]
pub struct Offer<'a> {
    endpoint: &'a mut Endpoint,
    id: u64,
    delivery: Delivery,
}
impl Offer<'_> {
    /// Actual offer correlation, without accepting custody or authorizing execution.
    pub fn offer_id(&self) -> u64 {
        self.id
    }
    pub fn delivery(&self) -> &Delivery {
        &self.delivery
    }
    pub fn accept(self) -> Result<Delivery, AcceptError> {
        let Self {
            endpoint,
            id,
            delivery,
        } = self;
        let result = frame(
            ACCEPTED,
            id,
            delivery.manifest.generation.as_str(),
            0,
            &[],
            endpoint.limits,
        )
        .and_then(|bytes| rights::send(&endpoint.socket, &bytes, &[]).map_err(Error::Io));
        match result {
            Ok(()) => {
                endpoint.poisoned = false;
                Ok(delivery)
            }
            Err(error) => Err(AcceptError {
                error,
                delivery: Box::new(delivery),
            }),
        }
    }
    /// Drop this offer's inventory BEFORE submitting a correlated NACK. The
    /// application must separately retire any copies it made while validating;
    /// rejection is not global revocation of descriptor references.
    pub fn reject(self) -> Result<(), Error> {
        let Self {
            endpoint,
            id,
            delivery,
        } = self;
        let generation = delivery.manifest.generation.clone();
        drop(delivery);
        let bytes = frame(REJECTED, id, generation.as_str(), 0, &[], endpoint.limits)?;
        rights::send(&endpoint.socket, &bytes, &[])?;
        endpoint.poisoned = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handle_off_bundle::{Bundle, BundleId, MANIFEST_VERSION, Member, MemberId};
    use handle_off_handles::ResourceId;
    use std::{
        fs::File,
        io::{Read, Write},
        os::unix::net::UnixStream,
        thread,
        time::Duration,
    };
    fn manifest(names: &[&str]) -> Manifest {
        Manifest {
            version: MANIFEST_VERSION,
            generation: GenerationId::new("generation-1").unwrap(),
            resources: names
                .iter()
                .map(|name| ResourceId::new(*name).unwrap())
                .collect(),
            bundles: names
                .iter()
                .enumerate()
                .map(|(index, name)| Bundle {
                    id: BundleId::new(format!("bundle-{index}")).unwrap(),
                    members: vec![Member {
                        id: MemberId::new("resource").unwrap(),
                        resource: ResourceId::new(*name).unwrap(),
                        extensions: vec![],
                    }],
                    extensions: vec![],
                })
                .collect(),
            extensions: vec![],
        }
    }
    fn handles(manifest: &Manifest) -> OwnedHandles {
        let mut table = OwnedHandles::default();
        for id in &manifest.resources {
            table
                .insert(id.clone(), File::open("/dev/null").unwrap().into())
                .unwrap();
        }
        table
    }
    fn pair() -> (UnixDatagram, UnixDatagram) {
        let (first, second) = UnixDatagram::pair().unwrap();
        for socket in [&first, &second] {
            socket
                .set_read_timeout(Some(Duration::from_millis(250)))
                .unwrap();
        }
        (first, second)
    }
    fn packet(manifest: &Manifest, id: u64, count: usize) -> Vec<u8> {
        frame(
            OFFER,
            id,
            manifest.generation.as_str(),
            count,
            &codec::encode_manifest(manifest, Limits::default().codec()).unwrap(),
            Limits::default(),
        )
        .unwrap()
    }
    fn rejected_packet(bytes: &[u8], count: usize, limits: Limits) -> Error {
        let (sender, receiver) = pair();
        let (socket, mut peer) = UnixStream::pair().unwrap();
        let fds: Vec<_> = (0..count).map(|_| socket.as_fd()).collect();
        rights::send(&sender, bytes, &fds).unwrap();
        let mut endpoint = Endpoint::new(receiver, limits).unwrap();
        let error = endpoint.receive().unwrap_err();
        assert!(endpoint.is_poisoned());
        // Sender's original is still usable; closing it yields EOF iff the entire
        // rejected offer's acquired FD inventory was cleaned up without leakage.
        assert!(socket.peer_addr().is_ok());
        drop(socket);
        peer.set_nonblocking(true).unwrap();
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
        error
    }
    #[test]
    fn socketpair_delivery_and_explicit_rejection_keep_original_owners() {
        let (sender, receiver) = pair();
        let worker = thread::spawn(move || {
            let mut endpoint = Endpoint::new(receiver, Limits::default()).unwrap();
            let offer = endpoint.receive().unwrap();
            assert_eq!(offer.delivery().manifest.bundles.len(), 2);
            let accepted = offer.accept().unwrap();
            assert_eq!(accepted.handles.len(), 2);
            endpoint.receive().unwrap().reject().unwrap();
            endpoint.receive().unwrap().accept().unwrap()
        });
        let manifest = manifest(&["first", "second"]);
        let table = handles(&manifest);
        let mut endpoint = Endpoint::new(sender, Limits::default()).unwrap();
        assert_eq!(endpoint.send(&manifest, &table).unwrap().offer_id, 1);
        let error = endpoint.send(&manifest, &table).unwrap_err();
        assert_eq!(error.phase, SendPhase::Rejected);
        assert!(!endpoint.is_poisoned());
        assert_eq!(endpoint.send(&manifest, &table).unwrap().offer_id, 3);
        drop(worker.join().unwrap());
        assert_eq!(table.len(), 2);
        let mut file = File::from(
            table
                .try_clone_to_owned()
                .unwrap()
                .remove(&manifest.resources[0])
                .unwrap(),
        );
        assert_eq!(file.read(&mut [0]).unwrap(), 0);
    }
    #[test]
    fn version_framing_generation_and_inventory_fail_closed() {
        let manifest = manifest(&["resource"]);
        let original = packet(&manifest, 1, 1);
        let mut version = original.clone();
        version[8..10].copy_from_slice(&999u16.to_le_bytes());
        assert!(matches!(
            rejected_packet(&version, 1, Limits::default()),
            Error::UnsupportedVersion(999)
        ));
        let mut bad_magic = original.clone();
        bad_magic[0] ^= 1;
        assert!(matches!(
            rejected_packet(&bad_magic, 1, Limits::default()),
            Error::InvalidFrame
        ));
        let mut reserved = original.clone();
        reserved[11] = 1;
        assert!(matches!(
            rejected_packet(&reserved, 1, Limits::default()),
            Error::InvalidFrame
        ));
        let mut trailing = original.clone();
        trailing.push(0);
        assert!(matches!(
            rejected_packet(&trailing, 1, Limits::default()),
            Error::InvalidFrame
        ));
        let mut generation = original.clone();
        generation[HEADER] = b'X';
        assert!(matches!(
            rejected_packet(&generation, 1, Limits::default()),
            Error::GenerationMismatch
        ));
        assert!(matches!(
            rejected_packet(&original, 0, Limits::default()),
            Error::DescriptorCountMismatch
        ));
        assert!(matches!(
            rejected_packet(&packet(&manifest, 1, 2), 2, Limits::default()),
            Error::DescriptorCountMismatch
        ));
        let mut invalid_body = original;
        invalid_body[HEADER + manifest.generation.as_str().len()..].fill(0xff);
        assert!(matches!(
            rejected_packet(&invalid_body, 1, Limits::default()),
            Error::Codec(_)
        ));
    }
    #[test]
    fn duplicate_resource_mapping_and_ancillary_or_payload_truncation_cleanup() {
        let manifest = manifest(&["resourceaaaa", "resourcebbbb"]);
        let mut bytes = packet(&manifest, 1, 2);
        // Same-length UTF-8 change preserves byte-valid archive structure but
        // corrupts semantic identity; there is deliberately no checksum here.
        for index in HEADER..bytes.len() - 12 {
            if &bytes[index..index + 12] == b"resourceaaaa" {
                bytes[index..index + 12].copy_from_slice(b"resourcebbbb");
            }
        }
        assert!(matches!(
            rejected_packet(&bytes, 2, Limits::default()),
            Error::Codec(CodecError::Manifest(ManifestError::DuplicateResource(_)))
        ));
        let valid = packet(&manifest, 1, 2);
        assert!(matches!(
            rejected_packet(
                &valid,
                2,
                Limits {
                    max_fds: 0,
                    ..Limits::default()
                }
            ),
            Error::Io(_)
        ));
        assert!(matches!(
            rejected_packet(
                &valid,
                2,
                Limits {
                    max_datagram: 64,
                    ..Limits::default()
                }
            ),
            Error::Io(_)
        ));
    }
    #[test]
    fn dropping_valid_offer_does_not_ack_and_duplicate_offer_ids_are_rejected() {
        let (sender, receiver) = pair();
        let manifest = manifest(&["resource"]);
        let (socket, mut peer) = UnixStream::pair().unwrap();
        rights::send(&sender, &packet(&manifest, 1, 1), &[socket.as_fd()]).unwrap();
        let mut endpoint = Endpoint::new(receiver, Limits::default()).unwrap();
        drop(endpoint.receive().unwrap());
        assert!(endpoint.is_poisoned());
        drop(socket);
        peer.set_nonblocking(true).unwrap();
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
        sender.set_nonblocking(true).unwrap();
        assert_eq!(
            rights::receive(
                &sender,
                ReceiveLimits {
                    max_payload: 1024,
                    max_fds: 0
                }
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::WouldBlock
        );

        let (sender, receiver) = pair();
        let mut endpoint = Endpoint::new(receiver, Limits::default()).unwrap();
        let file = File::open("/dev/null").unwrap();
        rights::send(&sender, &packet(&manifest, 1, 1), &[file.as_fd()]).unwrap();
        drop(endpoint.receive().unwrap().accept().unwrap());
        rights::receive(
            &sender,
            ReceiveLimits {
                max_payload: 1024,
                max_fds: 0,
            },
        )
        .unwrap();
        rights::send(&sender, &packet(&manifest, 1, 1), &[file.as_fd()]).unwrap();
        assert!(matches!(
            endpoint.receive().unwrap_err(),
            Error::DuplicateOffer
        ));
    }
    #[test]
    fn unrelated_and_duplicate_acknowledgments_do_not_confirm_delivery() {
        for case in ["id", "generation", "duplicate"] {
            let (sender, peer) = pair();
            let worker = thread::spawn(move || {
                let offer = rights::receive(&peer, Limits::default().receive()).unwrap();
                let offer = parse(&offer.payload).unwrap();
                let ack = frame(
                    ACCEPTED,
                    if case == "id" { offer.id + 1 } else { offer.id },
                    if case == "generation" {
                        "unrelated-generation"
                    } else {
                        offer.generation
                    },
                    0,
                    &[],
                    Limits::default(),
                )
                .unwrap();
                rights::send(&peer, &ack, &[]).unwrap();
                if case == "duplicate" {
                    rights::send(&peer, &ack, &[]).unwrap();
                    // Keep the peer alive until the next offer is queued.
                    rights::receive(&peer, Limits::default().receive()).unwrap();
                }
            });
            let manifest = manifest(&["resource"]);
            let table = handles(&manifest);
            let mut endpoint = Endpoint::new(sender, Limits::default()).unwrap();
            if case == "duplicate" {
                endpoint.send(&manifest, &table).unwrap();
            }
            let error = endpoint.send(&manifest, &table).unwrap_err();
            assert_eq!(error.phase, SendPhase::Indeterminate);
            assert!(matches!(error.error, Error::UnrelatedResponse));
            assert!(endpoint.is_poisoned());
            worker.join().unwrap();
        }
    }
    #[test]
    fn preflight_timeout_disconnect_and_failed_ack_preserve_correct_ownership() {
        let manifest = manifest(&["resource"]);
        let (sender, peer) = pair();
        let mut endpoint = Endpoint::new(sender, Limits::default()).unwrap();
        let error = endpoint
            .send(&manifest, &OwnedHandles::default())
            .unwrap_err();
        assert_eq!(error.phase, SendPhase::BeforeSend);
        assert!(!endpoint.is_poisoned());
        let error = endpoint.send(&manifest, &handles(&manifest)).unwrap_err();
        assert_eq!(error.phase, SendPhase::Indeterminate);
        assert!(matches!(error.error, Error::Io(_)));
        // Submitted references can still be queued at the peer despite timeout.
        assert_eq!(
            rights::receive(&peer, Limits::default().receive())
                .unwrap()
                .descriptors
                .len(),
            1
        );
        let (sender, peer) = pair();
        drop(peer);
        let mut endpoint = Endpoint::new(sender, Limits::default()).unwrap();
        assert_eq!(
            endpoint
                .send(&manifest, &handles(&manifest))
                .unwrap_err()
                .phase,
            SendPhase::Indeterminate
        );

        let (sender, receiver) = pair();
        let (socket, mut peer) = UnixStream::pair().unwrap();
        rights::send(&sender, &packet(&manifest, 1, 1), &[socket.as_fd()]).unwrap();
        let mut endpoint = Endpoint::new(receiver, Limits::default()).unwrap();
        let offer = endpoint.receive().unwrap();
        drop(sender);
        drop(socket);
        let error = offer.accept().unwrap_err();
        assert!(endpoint.is_poisoned());
        // ACK failure returns the accepted inventory rather than silently discarding it.
        let id = &manifest.resources[0];
        let mut copy = error.delivery.handles.try_clone_to_owned().unwrap();
        let mut socket = UnixStream::from(copy.remove(id).unwrap());
        peer.write_all(b"alive").unwrap();
        let mut bytes = [0; 5];
        socket.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"alive");
        drop(socket);
        drop(error);
        peer.set_nonblocking(true).unwrap();
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    }
    #[test]
    fn ack_with_attached_rights_is_rejected_and_cleaned_up() {
        let (sender, peer) = pair();
        let (socket, mut observer) = UnixStream::pair().unwrap();
        let worker = thread::spawn(move || {
            let offer = rights::receive(&peer, Limits::default().receive()).unwrap();
            let packet = parse(&offer.payload).unwrap();
            let ack = frame(
                ACCEPTED,
                packet.id,
                packet.generation,
                0,
                &[],
                Limits::default(),
            )
            .unwrap();
            rights::send(&peer, &ack, &[socket.as_fd()]).unwrap();
        });
        let manifest = manifest(&["resource"]);
        let table = handles(&manifest);
        let mut endpoint = Endpoint::new(sender, Limits::default()).unwrap();
        assert_eq!(
            endpoint.send(&manifest, &table).unwrap_err().phase,
            SendPhase::Indeterminate
        );
        worker.join().unwrap();
        observer.set_nonblocking(true).unwrap();
        assert_eq!(observer.read(&mut [0]).unwrap(), 0);
    }
}
