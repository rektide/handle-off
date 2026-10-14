use handle_off_handover::{GenerationId, ReleaseGrant};
use handle_off_transport_unix::{self as transport, Delivery, Offer, Receipt};
use handle_off_unix_rights as rights;
use std::{
    error::Error as StdError,
    fmt, io,
    os::unix::net::UnixDatagram,
    time::{Duration, Instant},
};
const MAGIC: &[u8; 8] = b"HODIR01\0";
pub const CONTROL_VERSION: u16 = 1;
pub const MAX_GENERATION_BYTES: usize = 256;
const HEADER: usize = 40;
const READY: u8 = 1;
const RELEASE: u8 = 2;
const RELEASED: u8 = 3;
const ABORT: u8 = 4;
const RETIRED: u8 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correlation {
    pub session: [u8; 16],
    pub generation: GenerationId,
    pub offer_id: u64,
}
impl Correlation {
    pub fn new(session: [u8; 16], generation: GenerationId, offer_id: u64) -> Result<Self, Error> {
        if offer_id == 0 || generation.as_str().len() > MAX_GENERATION_BYTES {
            return Err(Error::InvalidCorrelation);
        }
        Ok(Self {
            session,
            generation,
            offer_id,
        })
    }
}
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Delivery(transport::Error),
    InvalidCorrelation,
    InvalidFrame,
    UnsupportedVersion(u16),
    Unrelated,
    Unexpected,
    Poisoned,
    Spent,
    NotReady,
    WrongGeneration,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "direct handover control error: {self:?}")
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Delivery(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
fn encode(kind: u8, key: &Correlation) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER + key.generation.as_str().len());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&CONTROL_VERSION.to_le_bytes());
    bytes.push(kind);
    bytes.push(0);
    bytes.extend_from_slice(&key.session);
    bytes.extend_from_slice(&key.offer_id.to_le_bytes());
    bytes.extend_from_slice(&(key.generation.as_str().len() as u16).to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes.extend_from_slice(key.generation.as_str().as_bytes());
    bytes
}
fn decode(bytes: &[u8], key: &Correlation) -> Result<u8, Error> {
    if bytes.len() < HEADER || &bytes[..8] != MAGIC {
        return Err(Error::InvalidFrame);
    }
    let version = u16::from_le_bytes(bytes[8..10].try_into().map_err(|_| Error::InvalidFrame)?);
    if version != CONTROL_VERSION {
        return Err(Error::UnsupportedVersion(version));
    }
    let length =
        u16::from_le_bytes(bytes[36..38].try_into().map_err(|_| Error::InvalidFrame)?) as usize;
    if bytes[11] != 0
        || bytes[38..40] != [0, 0]
        || length == 0
        || length > MAX_GENERATION_BYTES
        || bytes.len() != HEADER + length
        || !matches!(bytes[10], READY | RELEASE | RELEASED | ABORT | RETIRED)
    {
        return Err(Error::InvalidFrame);
    }
    let id = u64::from_le_bytes(bytes[28..36].try_into().map_err(|_| Error::InvalidFrame)?);
    let generation = std::str::from_utf8(&bytes[HEADER..]).map_err(|_| Error::InvalidFrame)?;
    if id == 0 || generation.chars().any(char::is_control) {
        return Err(Error::InvalidFrame);
    }
    if bytes[12..28] != key.session
        || id != key.offer_id
        || &bytes[HEADER..] != key.generation.as_str().as_bytes()
    {
        return Err(Error::Unrelated);
    }
    Ok(bytes[10])
}
struct Channel {
    socket: UnixDatagram,
    key: Correlation,
    timeout: Duration,
    poisoned: bool,
    spent: bool,
}
impl Channel {
    fn new(socket: UnixDatagram, key: Correlation, timeout: Duration) -> Result<Self, Error> {
        Correlation::new(key.session, key.generation.clone(), key.offer_id)?;
        if timeout.is_zero() || Instant::now().checked_add(timeout).is_none() {
            return Err(Error::InvalidCorrelation);
        }
        socket.peer_addr()?;
        socket.set_read_timeout(Some(timeout))?;
        socket.set_write_timeout(Some(timeout))?;
        Ok(Self {
            socket,
            key,
            timeout,
            poisoned: false,
            spent: false,
        })
    }
    fn begin(&mut self) -> Result<Instant, Error> {
        if self.poisoned {
            return Err(Error::Poisoned);
        }
        if self.spent {
            return Err(Error::Spent);
        }
        self.poisoned = true;
        Instant::now()
            .checked_add(self.timeout)
            .ok_or(Error::InvalidCorrelation)
    }
    fn remaining(deadline: Instant) -> Result<Duration, Error> {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| {
                Error::Io(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "control deadline expired; outcome uncertain",
                ))
            })
    }
    fn send(&self, kind: u8, deadline: Instant) -> Result<(), Error> {
        self.socket
            .set_write_timeout(Some(Self::remaining(deadline)?))?;
        rights::send(&self.socket, &encode(kind, &self.key), &[])?;
        Ok(())
    }
    fn receive(&self, deadline: Instant) -> Result<u8, Error> {
        self.socket
            .set_read_timeout(Some(Self::remaining(deadline)?))?;
        let message = rights::receive(
            &self.socket,
            rights::ReceiveLimits {
                max_payload: HEADER + MAX_GENERATION_BYTES,
                max_fds: 0,
            },
        )?;
        decode(&message.payload, &self.key)
    }
}
#[derive(Debug)]
pub struct ReadyReceipt {
    pub correlation: Correlation,
}
#[derive(Debug)]
pub struct RetirementReceipt {
    pub correlation: Correlation,
}
#[derive(Debug)]
pub struct ReleaseReceipt {
    pub correlation: Correlation,
}
#[derive(Debug)]
pub struct ReleaseFailure {
    pub error: Error,
    pub grant: ReleaseGrant,
}
impl fmt::Display for ReleaseFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl StdError for ReleaseFailure {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(&self.error)
    }
}
pub struct Sender {
    channel: Channel,
    ready: bool,
}
impl Sender {
    pub fn new(
        control: UnixDatagram,
        correlation: Correlation,
        timeout: Duration,
    ) -> Result<Self, Error> {
        Ok(Self {
            channel: Channel::new(control, correlation, timeout)?,
            ready: false,
        })
    }
    pub fn correlation(&self) -> &Correlation {
        &self.channel.key
    }
    pub fn is_poisoned(&self) -> bool {
        self.channel.poisoned
    }
    pub fn wait_ready(&mut self, delivery: &Receipt) -> Result<ReadyReceipt, Error> {
        if delivery.offer_id != self.channel.key.offer_id
            || delivery.generation != self.channel.key.generation
        {
            return Err(Error::Unrelated);
        }
        if self.ready {
            return Err(Error::Unexpected);
        }
        let deadline = self.channel.begin()?;
        if self.channel.receive(deadline)? != READY {
            return Err(Error::Unexpected);
        }
        self.ready = true;
        self.channel.poisoned = false;
        Ok(ReadyReceipt {
            correlation: self.channel.key.clone(),
        })
    }
    /// Retirement evidence only, not old-owner permission. Use in core Attempt::abort.
    /// One preceding READY is allowed if abort raced lost bundle acceptance/ACK.
    pub fn abort(&mut self) -> Result<RetirementReceipt, Error> {
        let deadline = self.channel.begin()?;
        self.channel.spent = true;
        self.channel.send(ABORT, deadline)?;
        let mut kind = self.channel.receive(deadline)?;
        if kind == READY && !self.ready {
            self.ready = true;
            kind = self.channel.receive(deadline)?;
        }
        if kind != RETIRED {
            return Err(Error::Unexpected);
        }
        self.channel.poisoned = false;
        Ok(RetirementReceipt {
            correlation: self.channel.key.clone(),
        })
    }
    /// Only core can construct this grant after irreversible relinquishment.
    /// Failure retains the original one-shot approval, never grants old resume.
    pub fn release(&mut self, grant: ReleaseGrant) -> Result<ReleaseReceipt, Box<ReleaseFailure>> {
        let result = (|| {
            if grant.generation() != &self.channel.key.generation {
                return Err(Error::WrongGeneration);
            }
            if !self.ready {
                return Err(Error::NotReady);
            }
            let deadline = self.channel.begin()?;
            self.channel.spent = true;
            self.channel.send(RELEASE, deadline)?;
            if self.channel.receive(deadline)? != RELEASED {
                return Err(Error::Unexpected);
            }
            self.channel.poisoned = false;
            Ok(ReleaseReceipt {
                correlation: self.channel.key.clone(),
            })
        })();
        result.map_err(|error| Box::new(ReleaseFailure { error, grant }))
    }
}
#[derive(Debug)]
pub struct StagedDelivery {
    key: Correlation,
    delivery: Delivery,
}
impl StagedDelivery {
    pub fn delivery(&self) -> &Delivery {
        &self.delivery
    }
    pub fn correlation(&self) -> &Correlation {
        &self.key
    }
}
#[derive(Debug)]
pub enum RetainedStage<'a> {
    Unaccepted(Offer<'a>),
    Accepted(StagedDelivery),
}
#[derive(Debug)]
pub struct StageFailure<'a> {
    pub error: Error,
    pub retained: RetainedStage<'a>,
}
#[derive(Debug)]
pub enum ReadyError<E> {
    Application(E),
    Control(Error),
}
#[derive(Debug)]
pub struct ReleaseNotice {
    key: Correlation,
}
impl ReleaseNotice {
    pub fn correlation(&self) -> &Correlation {
        &self.key
    }
}
#[derive(Debug)]
pub struct AbortNotice {
    key: Correlation,
}
#[derive(Debug)]
pub enum Decision {
    Release(ReleaseNotice),
    Abort(AbortNotice),
}
#[derive(Debug)]
pub struct ActivatedDelivery {
    pub delivery: Delivery,
    pub permission: ReleaseNotice,
}
#[derive(Debug)]
pub struct ActivationFailure {
    pub error: Error,
    pub staged: StagedDelivery,
    pub permission: ReleaseNotice,
}
#[derive(Debug)]
pub enum RetirementError<E> {
    Application(E),
    Control(Error),
}
#[derive(Debug)]
pub struct RetirementFailure<E> {
    pub error: RetirementError<E>,
    pub retained: Option<StagedDelivery>,
    pub notice: AbortNotice,
}
pub struct Receiver {
    channel: Channel,
    staged: bool,
    ready: bool,
}
impl Receiver {
    pub fn new(
        control: UnixDatagram,
        correlation: Correlation,
        timeout: Duration,
    ) -> Result<Self, Error> {
        Ok(Self {
            channel: Channel::new(control, correlation, timeout)?,
            staged: false,
            ready: false,
        })
    }
    pub fn is_poisoned(&self) -> bool {
        self.channel.poisoned
    }
    /// Accept library custody only. Caller owns any application partial restore state.
    /// Failure returns the unaccepted Offer or accepted Delivery, never loses it.
    pub fn stage<'a>(&mut self, offer: Offer<'a>) -> Result<StagedDelivery, Box<StageFailure<'a>>> {
        let error = if self.staged || self.channel.spent {
            Some(Error::Spent)
        } else if self.channel.poisoned {
            Some(Error::Poisoned)
        } else if offer.offer_id() != self.channel.key.offer_id
            || offer.delivery().manifest.generation != self.channel.key.generation
        {
            Some(Error::Unrelated)
        } else {
            None
        };
        if let Some(error) = error {
            return Err(Box::new(StageFailure {
                error,
                retained: RetainedStage::Unaccepted(offer),
            }));
        }
        self.staged = true;
        match offer.accept() {
            Ok(delivery) => Ok(StagedDelivery {
                key: self.channel.key.clone(),
                delivery,
            }),
            Err(error) => Err(Box::new(StageFailure {
                error: Error::Delivery(error.error),
                retained: RetainedStage::Accepted(StagedDelivery {
                    key: self.channel.key.clone(),
                    delivery: *error.delivery,
                }),
            })),
        }
    }
    /// Callback must validate/restore dormant application state without shared I/O
    /// side effects. Partial app state stays caller-owned on any error. No READY on Err.
    pub fn ready<E>(
        &mut self,
        staged: &StagedDelivery,
        validate_restore: impl FnOnce(&Delivery) -> Result<(), E>,
    ) -> Result<(), ReadyError<E>> {
        if staged.key != self.channel.key {
            return Err(ReadyError::Control(Error::Unrelated));
        }
        if self.ready {
            return Err(ReadyError::Control(Error::Unexpected));
        }
        if !self.staged {
            return Err(ReadyError::Control(Error::NotReady));
        }
        if self.channel.poisoned {
            return Err(ReadyError::Control(Error::Poisoned));
        }
        if self.channel.spent {
            return Err(ReadyError::Control(Error::Spent));
        }
        validate_restore(&staged.delivery).map_err(ReadyError::Application)?;
        let deadline = self.channel.begin().map_err(ReadyError::Control)?;
        self.channel
            .send(READY, deadline)
            .map_err(ReadyError::Control)?;
        self.channel.poisoned = false;
        self.ready = true;
        Ok(())
    }
    /// One terminal command per session. Returned notices cannot be cloned or forged
    /// through constructors. Invalid/duplicate/lost commands never authorize I/O.
    pub fn wait_decision(&mut self) -> Result<Decision, Error> {
        if !self.staged {
            return Err(Error::NotReady);
        }
        let deadline = self.channel.begin()?;
        self.channel.spent = true;
        let decision = match self.channel.receive(deadline)? {
            RELEASE if self.ready => Decision::Release(ReleaseNotice {
                key: self.channel.key.clone(),
            }),
            ABORT => Decision::Abort(AbortNotice {
                key: self.channel.key.clone(),
            }),
            _ => return Err(Error::Unexpected),
        };
        self.channel.poisoned = false;
        Ok(decision)
    }
    /// ACK only execution permission receipt, not command/application completion.
    /// On ACK failure the caller still owns received permission and every FD.
    pub fn activate(
        &mut self,
        staged: StagedDelivery,
        notice: ReleaseNotice,
    ) -> Result<ActivatedDelivery, Box<ActivationFailure>> {
        let result = (|| {
            if staged.key != self.channel.key || notice.key != self.channel.key {
                return Err(Error::Unrelated);
            }
            if !self.ready || !self.channel.spent {
                return Err(Error::NotReady);
            }
            if self.channel.poisoned {
                return Err(Error::Poisoned);
            }
            self.channel.poisoned = true;
            let deadline = Instant::now()
                .checked_add(self.channel.timeout)
                .ok_or(Error::InvalidCorrelation)?;
            self.channel.send(RELEASED, deadline)?;
            self.channel.poisoned = false;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(ActivatedDelivery {
                delivery: staged.delivery,
                permission: notice,
            }),
            Err(error) => Err(Box::new(ActivationFailure {
                error,
                staged,
                permission: notice,
            })),
        }
    }
    /// Callback success must retire ALL app-created copies/actors/restoration paths.
    /// Library Delivery is then dropped BEFORE RETIRED. Err retains library FDs; app
    /// parked state remains outside this API. No automatic retry or resume on Drop.
    pub fn retire<E>(
        &mut self,
        staged: StagedDelivery,
        notice: AbortNotice,
        retire_app: impl FnOnce() -> Result<(), E>,
    ) -> Result<RetirementReceipt, Box<RetirementFailure<E>>> {
        if staged.key != self.channel.key
            || notice.key != self.channel.key
            || !self.channel.spent
            || self.channel.poisoned
        {
            return Err(Box::new(RetirementFailure {
                error: RetirementError::Control(Error::Unrelated),
                retained: Some(staged),
                notice,
            }));
        }
        if let Err(error) = retire_app() {
            return Err(Box::new(RetirementFailure {
                error: RetirementError::Application(error),
                retained: Some(staged),
                notice,
            }));
        }
        drop(staged);
        self.channel.poisoned = true;
        let result = Instant::now()
            .checked_add(self.channel.timeout)
            .ok_or(Error::InvalidCorrelation)
            .and_then(|deadline| self.channel.send(RETIRED, deadline));
        match result {
            Ok(()) => {
                self.channel.poisoned = false;
                Ok(RetirementReceipt {
                    correlation: self.channel.key.clone(),
                })
            }
            Err(error) => Err(Box::new(RetirementFailure {
                error: RetirementError::Control(error),
                retained: None,
                notice,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handle_off_bundle::{Bundle, BundleId, MANIFEST_VERSION, Manifest, Member, MemberId};
    use handle_off_handles::{OwnedHandles, ResourceId};
    use handle_off_handover::{Authority, Evidence, ParkingController, RelinquishOutcome};
    use std::{
        io::Read,
        os::{
            fd::{AsFd, OwnedFd},
            unix::net::UnixStream,
        },
        thread,
    };
    fn key() -> Correlation {
        Correlation::new([0x42; 16], GenerationId::new("generation-1").unwrap(), 1).unwrap()
    }
    fn pair() -> (UnixDatagram, UnixDatagram) {
        UnixDatagram::pair().unwrap()
    }
    fn raw_send(socket: &UnixDatagram, kind: u8, key: &Correlation) {
        rights::send(socket, &encode(kind, key), &[]).unwrap();
    }
    fn raw_receive(socket: &UnixDatagram, key: &Correlation) -> u8 {
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        decode(
            &rights::receive(
                socket,
                rights::ReceiveLimits {
                    max_payload: 296,
                    max_fds: 0,
                },
            )
            .unwrap()
            .payload,
            key,
        )
        .unwrap()
    }
    fn evidence(value: &'static str) -> Evidence<&'static str> {
        Evidence::new(key().generation, value)
    }
    fn stage(fd: OwnedFd) -> StagedDelivery {
        let resource = ResourceId::new("stream").unwrap();
        let mut handles = OwnedHandles::default();
        handles.insert(resource.clone(), fd).unwrap();
        StagedDelivery {
            key: key(),
            delivery: Delivery {
                manifest: Manifest {
                    version: MANIFEST_VERSION,
                    generation: key().generation,
                    resources: vec![resource.clone()],
                    bundles: vec![Bundle {
                        id: BundleId::new("app").unwrap(),
                        members: vec![Member {
                            id: MemberId::new("stream").unwrap(),
                            resource,
                            extensions: vec![],
                        }],
                        extensions: vec![],
                    }],
                    extensions: vec![],
                },
                handles,
            },
        }
    }
    #[test]
    fn bad_version_malformed_wrong_generation_session_and_stale_offer_fail_closed() {
        for mode in 0..5 {
            let (local, peer) = pair();
            let mut sender = Sender::new(local, key(), Duration::from_millis(100)).unwrap();
            let mut foreign = key();
            if mode == 2 {
                foreign.generation = GenerationId::new("wrong-generation").unwrap();
            }
            if mode == 3 {
                foreign.session[0] ^= 1;
            }
            if mode == 4 {
                foreign.offer_id += 1;
            }
            let mut frame = encode(READY, &foreign);
            if mode == 0 {
                frame[8..10].copy_from_slice(&99u16.to_le_bytes());
            }
            if mode == 1 {
                frame[11] = 1;
            }
            rights::send(&peer, &frame, &[]).unwrap();
            let error = sender
                .wait_ready(&Receipt {
                    offer_id: 1,
                    generation: key().generation,
                })
                .unwrap_err();
            assert!(match mode {
                0 => matches!(error, Error::UnsupportedVersion(99)),
                1 => matches!(error, Error::InvalidFrame),
                _ => matches!(error, Error::Unrelated),
            });
            assert!(sender.is_poisoned());
            assert!(matches!(sender.abort(), Err(Error::Poisoned)));
        }
    }
    #[test]
    fn lost_retirement_ack_keeps_old_owner_parked_and_evidence_retained() {
        let (local, peer) = pair();
        let mut sender = Sender::new(local, key(), Duration::from_millis(40)).unwrap();
        raw_send(&peer, READY, &key());
        let mut controller = ParkingController::new_parked(key().generation);
        let mut attempt = controller.attempt().unwrap();
        attempt
            .publish(evidence("possible delivered refs"), || {
                Ok(evidence("bundle receipt"))
            })
            .unwrap();
        attempt
            .verify_custody(evidence("wait ready"), || {
                sender
                    .wait_ready(&Receipt {
                        offer_id: 1,
                        generation: key().generation,
                    })
                    .map(|_| evidence("dormant peer"))
                    .map_err(|_| evidence("uncertain"))
            })
            .unwrap();
        assert!(
            attempt
                .abort(evidence("retire session"), || sender
                    .abort()
                    .map(|_| evidence("retired"))
                    .map_err(|_| evidence("retirement ACK lost")))
                .is_err()
        );
        assert_eq!(raw_receive(&peer, &key()), ABORT);
        assert_eq!(attempt.status().authority, Authority::ParkedOld);
        assert!(attempt.status().quarantined);
        assert!(
            attempt
                .records()
                .iter()
                .any(|record| record.evidence.value == "retirement ACK lost")
        );
    }
    #[test]
    fn lost_release_ack_never_regrants_old_permission_and_retains_grant() {
        let (local, peer) = pair();
        let mut sender = Sender::new(local, key(), Duration::from_millis(40)).unwrap();
        raw_send(&peer, READY, &key());
        sender
            .wait_ready(&Receipt {
                offer_id: 1,
                generation: key().generation,
            })
            .unwrap();
        let mut controller = ParkingController::new_parked(key().generation);
        let mut attempt = controller.attempt().unwrap();
        attempt
            .publish(evidence("publish"), || Ok(evidence("delivered")))
            .unwrap();
        attempt
            .verify_custody(evidence("verify"), || Ok(evidence("ready receipt")))
            .unwrap();
        attempt
            .relinquish(evidence("irreversible gate"), || {
                RelinquishOutcome::Relinquished(evidence("old permanently disabled"))
            })
            .unwrap();
        let mut retained = None;
        assert!(
            attempt
                .release(evidence("release"), |grant| match sender.release(grant) {
                    Ok(_) => Ok(evidence("released")),
                    Err(error) => {
                        retained = Some(error);
                        Err(evidence("release ambiguous"))
                    }
                })
                .is_err()
        );
        assert_eq!(raw_receive(&peer, &key()), RELEASE);
        assert!(sender.is_poisoned());
        assert_eq!(
            retained.as_ref().unwrap().grant.generation(),
            &key().generation
        );
        assert_eq!(attempt.status().authority, Authority::NewReleaseIssued);
        assert!(
            attempt
                .abort(evidence("forbidden"), || Ok(evidence(
                    "cannot return old permission"
                )))
                .is_err()
        );
    }
    #[test]
    fn receiver_duplicate_release_is_not_a_second_execution_approval() {
        let (local, peer) = pair();
        let mut receiver = Receiver::new(local, key(), Duration::from_millis(100)).unwrap();
        let (socket, _observer) = UnixStream::pair().unwrap();
        let staged = stage(socket.into());
        receiver.staged = true;
        receiver.ready(&staged, |_| Ok::<(), ()>(())).unwrap();
        assert_eq!(raw_receive(&peer, &key()), READY);
        raw_send(&peer, RELEASE, &key());
        raw_send(&peer, RELEASE, &key());
        let Decision::Release(notice) = receiver.wait_decision().unwrap() else {
            panic!("expected release");
        };
        let activated = receiver.activate(staged, notice).unwrap();
        assert_eq!(raw_receive(&peer, &key()), RELEASED);
        assert_eq!(activated.permission.correlation(), &key());
        assert!(matches!(receiver.wait_decision(), Err(Error::Spent)));
    }
    #[test]
    fn retirement_failure_retains_delivery_and_app_copies_without_implicit_resume() {
        let (local, peer) = pair();
        let mut receiver = Receiver::new(local, key(), Duration::from_millis(100)).unwrap();
        let (socket, mut observer) = UnixStream::pair().unwrap();
        let staged = stage(socket.into());
        receiver.staged = true;
        let copy = staged
            .delivery
            .handles
            .iter()
            .next()
            .unwrap()
            .1
            .as_fd()
            .try_clone_to_owned()
            .unwrap();
        raw_send(&peer, ABORT, &key());
        let Decision::Abort(notice) = receiver.wait_decision().unwrap() else {
            panic!("expected abort");
        };
        let failure = receiver
            .retire(staged, notice, || Err("app copy retirement ambiguous"))
            .unwrap_err();
        assert!(failure.retained.is_some());
        drop(copy);
        observer.set_nonblocking(true).unwrap();
        assert_eq!(
            observer.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(failure);
        assert_eq!(observer.read(&mut [0]).unwrap(), 0);
        peer.set_nonblocking(true).unwrap();
        assert_eq!(
            rights::receive(
                &peer,
                rights::ReceiveLimits {
                    max_payload: 296,
                    max_fds: 0
                }
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::WouldBlock
        );
    }
    #[test]
    fn activation_ack_error_retains_received_permission_and_resource_custody() {
        let (local, peer) = pair();
        let mut receiver = Receiver::new(local, key(), Duration::from_millis(100)).unwrap();
        let (socket, mut observer) = UnixStream::pair().unwrap();
        let staged = stage(socket.into());
        receiver.staged = true;
        receiver.ready(&staged, |_| Ok::<(), ()>(())).unwrap();
        raw_receive(&peer, &key());
        raw_send(&peer, RELEASE, &key());
        let Decision::Release(notice) = receiver.wait_decision().unwrap() else {
            panic!("expected release");
        };
        drop(peer);
        let failure = receiver.activate(staged, notice).unwrap_err();
        assert_eq!(failure.permission.correlation(), &key());
        observer.set_nonblocking(true).unwrap();
        assert_eq!(
            observer.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(failure);
        assert_eq!(observer.read(&mut [0]).unwrap(), 0);
    }
    #[test]
    fn abort_can_cross_initial_ready_after_lost_bundle_ack() {
        let (local, peer) = pair();
        let mut sender = Sender::new(local, key(), Duration::from_secs(1)).unwrap();
        let worker = thread::spawn(move || {
            assert_eq!(raw_receive(&peer, &key()), ABORT);
            raw_send(&peer, READY, &key());
            raw_send(&peer, RETIRED, &key());
        });
        assert_eq!(sender.abort().unwrap().correlation, key());
        worker.join().unwrap();
    }
    #[test]
    fn control_fds_are_rejected_and_closed_and_timeout_cannot_be_ready() {
        let (local, peer) = pair();
        let mut sender = Sender::new(local, key(), Duration::from_millis(40)).unwrap();
        let (socket, mut observer) = UnixStream::pair().unwrap();
        rights::send(&peer, &encode(READY, &key()), &[socket.as_fd()]).unwrap();
        assert!(
            sender
                .wait_ready(&Receipt {
                    offer_id: 1,
                    generation: key().generation
                })
                .is_err()
        );
        drop(socket);
        observer.set_nonblocking(true).unwrap();
        assert_eq!(observer.read(&mut [0]).unwrap(), 0);
        let (local, _peer) = pair();
        let mut sender = Sender::new(local, key(), Duration::from_millis(40)).unwrap();
        assert!(matches!(
            sender.wait_ready(&Receipt {
                offer_id: 1,
                generation: key().generation
            }),
            Err(Error::Io(_))
        ));
        assert!(sender.is_poisoned());
    }
    #[test]
    fn partial_application_restore_error_keeps_copies_and_sends_no_ready() {
        let (local, peer) = pair();
        let mut receiver = Receiver::new(local, key(), Duration::from_millis(100)).unwrap();
        let (socket, mut observer) = UnixStream::pair().unwrap();
        let staged = stage(socket.into());
        receiver.staged = true;
        let mut partial = None;
        assert!(matches!(
            receiver.ready(&staged, |delivery| {
                partial = Some(
                    delivery
                        .handles
                        .iter()
                        .next()
                        .unwrap()
                        .1
                        .as_fd()
                        .try_clone_to_owned()
                        .unwrap(),
                );
                Err("partial restore failed, copy retained by app")
            }),
            Err(ReadyError::Application(_))
        ));
        peer.set_nonblocking(true).unwrap();
        assert_eq!(
            rights::receive(
                &peer,
                rights::ReceiveLimits {
                    max_payload: 296,
                    max_fds: 0
                }
            )
            .unwrap_err()
            .kind(),
            io::ErrorKind::WouldBlock
        );
        drop(staged);
        observer.set_nonblocking(true).unwrap();
        assert_eq!(
            observer.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(partial);
        assert_eq!(observer.read(&mut [0]).unwrap(), 0);
    }
}
