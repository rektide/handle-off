//! Cooperative handover bookkeeping, not an I/O owner or supervisor.
//!
//! Applications keep original resources/state in their own long-lived parking
//! controller. Embed this controller alongside them, and retain both until explicit
//! resolution or application shutdown. A prepared attempt only borrows bookkeeping:
//! Drop quarantines it without resume, resource closure, backend cleanup or callbacks.
//! Evidence remains in the controller. Evidence values should be passive metadata,
//! not auto-resuming application leases. The application must disarm such leases
//! BEFORE any publication begins (not merely before accepting a transport ACK).
//!
//! Contract: all conflicting old I/O/admission is parked, originals are retained
//! until verified abort or irreversible commit (and longer if recovery needs them),
//! and accepted successors stay dormant until release. Dormancy also forbids shared
//! state changes such as seek, O_NONBLOCK, socket shutdown or protocol writes, unless
//! explicitly accounted for by application rollback. Actual capture sufficiency,
//! application compatibility and backend verification belong to the application.
//! Generations must be unique per ownership domain; never reset bookkeeping to
//! bypass quarantine. Tokens sequence only cooperating API paths: public evidence
//! is an attestation, not authentication or proof of quiescence. External aliases,
//! independent actors and raw FD access cannot be policed by Rust tokens.
//!
//! Publication is marked exposed BEFORE its callback; errors/panics cannot establish
//! non-delivery. Custody verification must cover all resources and a dormant peer,
//! not a raw receipt/metadata match alone. Abort approval requires verified retirement
//! of all possibly delivered copies and restoration opportunities. Relinquishment
//! starts indeterminate; only an explicit NotRelinquished outcome restores parked
//! old authority. Once relinquished or indeterminate, no old resume approval exists.
//! Release authorizes the new owner BEFORE its callback; failed/escaped release is
//! irreversible, with no automatic retransmission, reissued grant or old resume.
#![forbid(unsafe_code)]

pub use handle_off_bundle::GenerationId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    /// Old owner is parked, not allowed I/O. Verified abort may approve its resume.
    ParkedOld,
    /// Old execution authority irrevocably surrendered; resources may remain retained.
    Relinquished,
    /// Outcome unknown: neither old resume nor new release may be approved.
    Indeterminate,
    /// One linear old-resume approval has been issued. Terminal for this generation.
    OldResumeIssued,
    /// New release approval escaped to the callback; it may already have caused I/O.
    NewReleaseIssued,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Custody {
    LocalOnly,
    Exposed,
    Verified,
    Retired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub authority: Authority,
    pub custody: Custody,
    pub quarantined: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Publish,
    VerifyCustody,
    Abort,
    Relinquish,
    ConfirmRelinquished,
    Release,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Started(Action),
    Succeeded(Action),
    Failed(Action),
    WrongGeneration(Action),
    Rejected(Action),
    NotRelinquished,
    Relinquished,
    Indeterminate,
}
/// Backend/application-specific evidence, never collapsed into an unrestricted
/// generic "safe to resume" boolean. Core checks the generation label only.
#[derive(Debug)]
pub struct Evidence<E> {
    pub generation: GenerationId,
    pub value: E,
}
impl<E> Evidence<E> {
    pub fn new(generation: GenerationId, value: E) -> Self {
        Self { generation, value }
    }
}
#[derive(Debug)]
pub struct Record<E> {
    pub attempt: u64,
    pub kind: RecordKind,
    pub evidence: Evidence<E>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Abandonment {
    pub attempt: u64,
    pub authority: Authority,
    pub custody: Custody,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionError {
    Terminal,
    AttemptIdExhausted,
    InvalidState { record: usize },
    WrongGeneration { record: usize },
    ActionFailed { record: usize },
}
impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "handover transition error: {self:?}")
    }
}
impl std::error::Error for TransitionError {}

/// Long-lived application-held bookkeeping. No original resources are owned here.
/// Dropping an Attempt preserves this state/evidence; dropping the controller is
/// explicit application disposal, never a rollback or implied permission to run.
#[derive(Debug)]
pub struct ParkingController<E> {
    generation: GenerationId,
    status: Status,
    records: Vec<Record<E>>,
    next_attempt: u64,
    abandoned_attempts: u64,
    last_abandonment: Option<Abandonment>,
}
impl<E> ParkingController<E> {
    /// Attests that the application is already parked under the module contract.
    /// Does not park threads, inspect resources or revoke an existing actor's rights.
    pub fn new_parked(generation: GenerationId) -> Self {
        Self {
            generation,
            status: Status {
                authority: Authority::ParkedOld,
                custody: Custody::LocalOnly,
                quarantined: false,
            },
            records: Vec::new(),
            next_attempt: 1,
            abandoned_attempts: 0,
            last_abandonment: None,
        }
    }
    pub fn generation(&self) -> &GenerationId {
        &self.generation
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn records(&self) -> &[Record<E>] {
        &self.records
    }
    pub fn abandoned_attempts(&self) -> u64 {
        self.abandoned_attempts
    }
    pub fn last_abandonment(&self) -> Option<Abandonment> {
        self.last_abandonment
    }
    /// Explicitly start or recover an attempt; quarantine/evidence and permissions
    /// are NOT reset. Borrowing excludes another simultaneous attempt on this controller.
    pub fn attempt(&mut self) -> Result<Attempt<'_, E>, TransitionError> {
        if matches!(
            self.status.authority,
            Authority::OldResumeIssued | Authority::NewReleaseIssued
        ) {
            return Err(TransitionError::Terminal);
        }
        let id = self.next_attempt;
        self.next_attempt = id
            .checked_add(1)
            .ok_or(TransitionError::AttemptIdExhausted)?;
        Ok(Attempt {
            controller: self,
            id,
            completed: false,
        })
    }
}

#[must_use = "a dropped attempt quarantines; retain the controller and explicitly resolve it"]
pub struct Attempt<'a, E> {
    controller: &'a mut ParkingController<E>,
    id: u64,
    completed: bool,
}
impl<E> Drop for Attempt<'_, E> {
    fn drop(&mut self) {
        if !self.completed {
            self.controller.status.quarantined = true;
            self.controller.abandoned_attempts =
                self.controller.abandoned_attempts.saturating_add(1);
            self.controller.last_abandonment = Some(Abandonment {
                attempt: self.id,
                authority: self.controller.status.authority,
                custody: self.controller.status.custody,
            });
        }
    }
}
/// Approvals are non-Clone and have no public constructors or automatic Drop effects.
///
/// ```compile_fail
/// use handle_off_handover::ResumePermit;
/// fn duplicate(permit: ResumePermit) { let _ = permit.clone(); }
/// ```
#[must_use = "resume approval must be explicitly consumed by the matching parked application"]
#[derive(Debug)]
pub struct ResumePermit {
    generation: GenerationId,
}
impl ResumePermit {
    pub fn generation(&self) -> &GenerationId {
        &self.generation
    }
}
#[must_use = "release is an irreversible approval, not a delivery acknowledgment"]
#[derive(Debug)]
pub struct ReleaseGrant {
    generation: GenerationId,
}
impl ReleaseGrant {
    pub fn generation(&self) -> &GenerationId {
        &self.generation
    }
}
#[derive(Debug)]
pub enum RelinquishOutcome<E> {
    NotRelinquished(Evidence<E>),
    Relinquished(Evidence<E>),
    Indeterminate(Evidence<E>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relinquishment {
    NotRelinquished,
    Relinquished,
    Indeterminate,
}

impl<E> Attempt<'_, E> {
    pub fn generation(&self) -> &GenerationId {
        &self.controller.generation
    }
    pub fn status(&self) -> Status {
        self.controller.status
    }
    pub fn records(&self) -> &[Record<E>] {
        &self.controller.records
    }
    fn record(&mut self, kind: RecordKind, evidence: Evidence<E>) -> usize {
        let index = self.controller.records.len();
        self.controller.records.push(Record {
            attempt: self.id,
            kind,
            evidence,
        });
        index
    }
    fn check_evidence(
        &mut self,
        action: Action,
        kind: RecordKind,
        evidence: Evidence<E>,
    ) -> Result<usize, TransitionError> {
        let matches = evidence.generation == self.controller.generation;
        let index = self.record(
            if matches {
                kind
            } else {
                RecordKind::WrongGeneration(action)
            },
            evidence,
        );
        if !matches {
            self.controller.status.quarantined = true;
            return Err(TransitionError::WrongGeneration { record: index });
        }
        Ok(index)
    }
    fn start(
        &mut self,
        action: Action,
        valid: bool,
        evidence: Evidence<E>,
    ) -> Result<(), TransitionError> {
        if !valid {
            let record = self.record(RecordKind::Rejected(action), evidence);
            return Err(TransitionError::InvalidState { record });
        }
        self.check_evidence(action, RecordKind::Started(action), evidence)?;
        self.controller.status.quarantined = true;
        Ok(())
    }
    fn finish(
        &mut self,
        action: Action,
        result: Result<Evidence<E>, Evidence<E>>,
    ) -> Result<(), TransitionError> {
        match result {
            Ok(evidence) => {
                self.check_evidence(action, RecordKind::Succeeded(action), evidence)?;
                Ok(())
            }
            Err(evidence) => {
                let record = self.check_evidence(action, RecordKind::Failed(action), evidence)?;
                Err(TransitionError::ActionFailed { record })
            }
        }
    }
    /// Start evidence must retain the complete possible publication plan BEFORE
    /// side effects. A callback Err/panic remains exposed, even if no send occurred.
    pub fn publish(
        &mut self,
        start: Evidence<E>,
        operation: impl FnOnce() -> Result<Evidence<E>, Evidence<E>>,
    ) -> Result<(), TransitionError> {
        self.start(
            Action::Publish,
            self.status().authority == Authority::ParkedOld
                && self.status().custody == Custody::LocalOnly,
            start,
        )?;
        self.controller.status.custody = Custody::Exposed;
        self.finish(Action::Publish, operation())?;
        self.controller.status.quarantined = false;
        Ok(())
    }
    /// Success is an explicit backend attestation of generation-matched complete
    /// custody and application dormancy under its policy, not merely a transport ACK.
    pub fn verify_custody(
        &mut self,
        start: Evidence<E>,
        verify: impl FnOnce() -> Result<Evidence<E>, Evidence<E>>,
    ) -> Result<(), TransitionError> {
        self.start(
            Action::VerifyCustody,
            matches!(
                self.status().authority,
                Authority::ParkedOld | Authority::Relinquished | Authority::Indeterminate
            ) && self.status().custody != Custody::LocalOnly,
            start,
        )?;
        self.controller.status.custody = Custody::Exposed;
        self.finish(Action::VerifyCustody, verify())?;
        self.controller.status.custody = Custody::Verified;
        self.controller.status.quarantined = self.status().authority == Authority::Indeterminate;
        Ok(())
    }
    /// Success must mean ALL possible copies/restoration opportunities were retired
    /// and no successor may execute. Errors/panics retain exposure and cannot resume.
    /// This callback never resumes the application; the returned permit does that.
    pub fn abort(
        &mut self,
        start: Evidence<E>,
        retire: impl FnOnce() -> Result<Evidence<E>, Evidence<E>>,
    ) -> Result<ResumePermit, TransitionError> {
        self.start(
            Action::Abort,
            self.status().authority == Authority::ParkedOld,
            start,
        )?;
        self.controller.status.custody = Custody::Exposed;
        self.finish(Action::Abort, retire())?;
        self.controller.status.custody = Custody::Retired;
        self.controller.status.authority = Authority::OldResumeIssued;
        self.controller.status.quarantined = false;
        self.completed = true;
        Ok(ResumePermit {
            generation: self.controller.generation.clone(),
        })
    }
    /// Callback must permanently disable old execution (including lease Drop or
    /// shutdown paths) BEFORE attesting Relinquished. Ambiguous Errs must be mapped
    /// to Indeterminate; NotRelinquished is a stronger, explicit no-revocation claim.
    pub fn relinquish(
        &mut self,
        start: Evidence<E>,
        operation: impl FnOnce() -> RelinquishOutcome<E>,
    ) -> Result<Relinquishment, TransitionError> {
        self.start(
            Action::Relinquish,
            self.status().authority == Authority::ParkedOld
                && self.status().custody == Custody::Verified,
            start,
        )?;
        self.controller.status.authority = Authority::Indeterminate;
        let (result, kind, evidence) = match operation() {
            RelinquishOutcome::NotRelinquished(e) => (
                Relinquishment::NotRelinquished,
                RecordKind::NotRelinquished,
                e,
            ),
            RelinquishOutcome::Relinquished(e) => {
                (Relinquishment::Relinquished, RecordKind::Relinquished, e)
            }
            RelinquishOutcome::Indeterminate(e) => {
                (Relinquishment::Indeterminate, RecordKind::Indeterminate, e)
            }
        };
        self.check_evidence(Action::Relinquish, kind, evidence)?;
        self.controller.status.authority = match result {
            Relinquishment::NotRelinquished => Authority::ParkedOld,
            Relinquishment::Relinquished => Authority::Relinquished,
            Relinquishment::Indeterminate => Authority::Indeterminate,
        };
        if result == Relinquishment::Relinquished {
            self.controller.status.quarantined = false;
        }
        Ok(result)
    }
    /// Explicit application evidence may resolve uncertainty only toward permanent
    /// relinquishment, never back into an old resume permission.
    pub fn confirm_relinquished(&mut self, evidence: Evidence<E>) -> Result<(), TransitionError> {
        self.start(
            Action::ConfirmRelinquished,
            self.status().authority == Authority::Indeterminate,
            evidence,
        )?;
        self.controller.status.authority = Authority::Relinquished;
        Ok(())
    }
    /// The non-Clone grant may escape to the backend. State is irreversible BEFORE
    /// invoking it. Err/panic cannot establish non-release and never reissues a grant.
    /// Any manual idempotent retry must use backend-retained original release data.
    pub fn release(
        &mut self,
        start: Evidence<E>,
        operation: impl FnOnce(ReleaseGrant) -> Result<Evidence<E>, Evidence<E>>,
    ) -> Result<(), TransitionError> {
        self.start(
            Action::Release,
            self.status().authority == Authority::Relinquished
                && self.status().custody == Custody::Verified,
            start,
        )?;
        self.controller.status.authority = Authority::NewReleaseIssued;
        let grant = ReleaseGrant {
            generation: self.controller.generation.clone(),
        };
        self.finish(Action::Release, operation(grant))?;
        self.controller.status.quarantined = false;
        self.completed = true;
        Ok(())
    }
}
