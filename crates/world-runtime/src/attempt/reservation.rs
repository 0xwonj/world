use core::fmt;
use core::num::NonZeroU32;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, SimMoment};
use world_model::ActionEvaluationInvocationId;

use crate::action_evaluation::{
    ActionEvaluationCaptureFingerprint, ActionEvaluationCaptureId, ActionEvaluationCaptureRequest,
};
use crate::authority::AuthorityCursor;
use crate::kernel::{
    AdmitRequest, InputId, InputRequestFingerprint, KernelSafetyCause, LedgerRetirement,
    ManageRequest, ManagementRequestFingerprint, ManagementRequestId, SessionManagement,
};
use crate::scheduler::SchedulerKey;

use super::binding::{AttemptBinding, RunAttemptId};
use super::disposition::AttemptDispositionId;

const RESERVED_OPERATION_SCHEMA_VERSION: u16 = 2;
const DUE_SET_FINGERPRINT_SCHEMA_VERSION: u16 = 2;
const ATTEMPT_STEP_SCHEMA_VERSION: u16 = 1;

const RESERVED_OPERATION_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("reserved-operation-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("reserved operation domain must be valid"),
    };

const DUE_SET_FINGERPRINT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("due-set-fingerprint-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("due-set fingerprint domain must be valid"),
    };

const ATTEMPT_STEP_DOMAIN: CanonicalDomain = match CanonicalDomain::new("attempt-step") {
    Ok(domain) => domain,
    Err(_) => panic!("attempt step domain must be valid"),
};

/// Repository-local grant that fences process capabilities for one reservation.
///
/// Reconciliation may release and later recreate the same logical step. The
/// grant distinguishes those authority instances without changing semantic
/// step, receipt, record, or trajectory identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReservationGrant(u64);

impl ReservationGrant {
    pub(crate) const FIRST: Self = Self(0);

    pub(crate) const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Complete concrete operation retained by one attempted world step.
///
/// Command and action-evaluation admission remain distinct protocols even
/// though both belong to the same authority-level admission family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReservedOperationDescriptor {
    AdmitCommand {
        id: InputId,
        fingerprint: InputRequestFingerprint,
        effective: SimMoment,
    },
    Fire {
        fired: SimMoment,
        resulting_frontier: SimMoment,
        due_count: NonZeroU32,
        due_set_fingerprint: DueSetFingerprint,
    },
    Manage {
        id: ManagementRequestId,
        fingerprint: ManagementRequestFingerprint,
        operation: SessionManagement,
    },
    KernelSafety {
        cause: KernelSafetyCause,
    },
    AdmitActionEvaluation {
        capture: ActionEvaluationCaptureId,
        fingerprint: ActionEvaluationCaptureFingerprint,
        invocation: ActionEvaluationInvocationId,
        effective: SimMoment,
    },
}

impl ReservedOperationDescriptor {
    pub(crate) fn admit_command(request: &AdmitRequest) -> Self {
        Self::AdmitCommand {
            id: request.id(),
            fingerprint: request.fingerprint(),
            effective: request.effective(),
        }
    }

    pub(crate) fn fire(
        fired: SimMoment,
        resulting_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<Self, DueSetFingerprintError> {
        let (due_count, due_set_fingerprint) =
            DueSetFingerprint::derive_checked(fired, resulting_frontier, due_keys)?;
        Ok(Self::Fire {
            fired,
            resulting_frontier,
            due_count,
            due_set_fingerprint,
        })
    }

    pub(crate) const fn manage(request: ManageRequest) -> Self {
        Self::Manage {
            id: request.id(),
            fingerprint: request.fingerprint(),
            operation: request.operation(),
        }
    }

    pub(crate) const fn kernel_safety(cause: KernelSafetyCause) -> Self {
        Self::KernelSafety { cause }
    }

    pub(crate) const fn admit_action_evaluation(request: &ActionEvaluationCaptureRequest) -> Self {
        Self::AdmitActionEvaluation {
            capture: request.capture(),
            fingerprint: request.fingerprint(),
            invocation: request.invocation(),
            effective: request.effective(),
        }
    }

    const fn canonical_tag(self) -> u32 {
        match self {
            Self::AdmitCommand { .. } => 0,
            Self::Fire { .. } => 1,
            Self::Manage { .. } => 2,
            Self::KernelSafety { .. } => 3,
            Self::AdmitActionEvaluation { .. } => 4,
        }
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        reserved_operation_bytes(self)
    }
}

/// Why an exact due scheduler-key sequence could not form a Fire identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DueSetFingerprintError {
    Empty,
    CountOverflow { count: usize },
}

impl fmt::Display for DueSetFingerprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a Fire due set cannot be empty"),
            Self::CountOverflow { count } => {
                write!(formatter, "Fire due-set count {count} exceeds u32")
            }
        }
    }
}

impl std::error::Error for DueSetFingerprintError {}

/// Fixed-width canonical identity of one exact ordered Fire due set.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DueSetFingerprint([u8; 32]);

impl DueSetFingerprint {
    pub(crate) fn derive_checked(
        fired: SimMoment,
        resulting_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<(NonZeroU32, Self), DueSetFingerprintError> {
        let due_count = checked_due_count(due_keys.len())?;
        Ok((
            due_count,
            Self::derive(fired, resulting_frontier, due_count, due_keys),
        ))
    }

    fn derive(
        fired: SimMoment,
        resulting_frontier: SimMoment,
        due_count: NonZeroU32,
        due_keys: &[SchedulerKey],
    ) -> Self {
        Self(
            ContentDigest::of_canonical(&due_set_fingerprint_bytes(
                fired,
                resulting_frontier,
                due_count,
                due_keys,
            ))
            .into_bytes(),
        )
    }

    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for DueSetFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for DueSetFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "DueSetFingerprint({self})")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ReservedOperationFingerprint([u8; 32]);

impl ReservedOperationFingerprint {
    fn derive(operation: ReservedOperationDescriptor) -> Self {
        Self(ContentDigest::of_canonical(&operation.canonical_bytes()).into_bytes())
    }

    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for ReservedOperationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for ReservedOperationFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ReservedOperationFingerprint({self})")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AttemptStepId([u8; 32]);

impl AttemptStepId {
    fn derive(
        attempt: RunAttemptId,
        expected: AuthorityCursor,
        operation: ReservedOperationFingerprint,
    ) -> Self {
        Self(
            ContentDigest::of_canonical(&attempt_step_bytes(attempt, expected, operation))
                .into_bytes(),
        )
    }

    #[cfg(test)]
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[cfg(test)]
    pub(crate) const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for AttemptStepId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for AttemptStepId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "AttemptStepId({self})")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReservationFailureAttachError {
    NotFire {
        operation: Box<ReservedOperationDescriptor>,
    },
    AlreadyAttached {
        existing: AttemptDispositionId,
    },
}

/// Durable evidence that one exact attempt operation owns the repository gate.
///
/// The repository's atomic state transition grants exclusion; this value
/// records what that transition reserved.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StepReservation {
    binding: AttemptBinding,
    grant: ReservationGrant,
    expected: AuthorityCursor,
    operation: ReservedOperationDescriptor,
    operation_fingerprint: ReservedOperationFingerprint,
    step: AttemptStepId,
    disposition: Option<AttemptDispositionId>,
}

impl StepReservation {
    pub(crate) fn new(
        binding: AttemptBinding,
        grant: ReservationGrant,
        expected: AuthorityCursor,
        operation: ReservedOperationDescriptor,
    ) -> Self {
        let operation_fingerprint = ReservedOperationFingerprint::derive(operation);
        Self {
            binding,
            grant,
            expected,
            operation,
            operation_fingerprint,
            step: AttemptStepId::derive(binding.attempt(), expected, operation_fingerprint),
            disposition: None,
        }
    }

    pub(crate) const fn binding(&self) -> AttemptBinding {
        self.binding
    }

    pub(crate) const fn grant(&self) -> ReservationGrant {
        self.grant
    }

    pub(crate) const fn expected(&self) -> AuthorityCursor {
        self.expected
    }

    pub(crate) const fn operation(&self) -> ReservedOperationDescriptor {
        self.operation
    }

    pub(crate) const fn operation_fingerprint(&self) -> ReservedOperationFingerprint {
        self.operation_fingerprint
    }

    pub(crate) const fn step(&self) -> AttemptStepId {
        self.step
    }

    pub(crate) const fn disposition(&self) -> Option<AttemptDispositionId> {
        self.disposition
    }

    pub(crate) fn attach_failure(
        &mut self,
        disposition: AttemptDispositionId,
    ) -> Result<(), ReservationFailureAttachError> {
        if !matches!(self.operation, ReservedOperationDescriptor::Fire { .. }) {
            return Err(ReservationFailureAttachError::NotFire {
                operation: Box::new(self.operation),
            });
        }

        match self.disposition {
            None => {
                self.disposition = Some(disposition);
                Ok(())
            }
            Some(existing) => Err(ReservationFailureAttachError::AlreadyAttached { existing }),
        }
    }
}

fn reserved_operation_bytes(operation: ReservedOperationDescriptor) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(RESERVED_OPERATION_DOMAIN);
    writer.write_u16(RESERVED_OPERATION_SCHEMA_VERSION);
    writer.write_discriminant(operation.canonical_tag());
    match operation {
        ReservedOperationDescriptor::AdmitCommand {
            id,
            fingerprint,
            effective,
        } => {
            writer.write_u64(id.get());
            write_fixed_bytes(&mut writer, fingerprint.as_bytes());
            write_moment(&mut writer, effective);
        }
        ReservedOperationDescriptor::Fire {
            fired,
            resulting_frontier,
            due_count,
            due_set_fingerprint,
        } => {
            write_moment(&mut writer, fired);
            write_moment(&mut writer, resulting_frontier);
            writer.write_u32(due_count.get());
            write_fixed_bytes(&mut writer, due_set_fingerprint.as_bytes());
        }
        ReservedOperationDescriptor::Manage {
            id,
            fingerprint,
            operation,
        } => {
            writer.write_u64(id.get());
            write_fixed_bytes(&mut writer, fingerprint.as_bytes());
            write_management_operation(&mut writer, operation);
        }
        ReservedOperationDescriptor::KernelSafety { cause } => {
            write_owned_bytes(&mut writer, cause.canonical_bytes().as_bytes());
        }
        ReservedOperationDescriptor::AdmitActionEvaluation {
            capture,
            fingerprint,
            invocation,
            effective,
        } => {
            writer.write_u64(capture.get());
            write_fixed_bytes(&mut writer, fingerprint.as_bytes());
            write_fixed_bytes(&mut writer, invocation.as_bytes());
            write_moment(&mut writer, effective);
        }
    }
    writer.finish()
}

fn checked_due_count(count: usize) -> Result<NonZeroU32, DueSetFingerprintError> {
    let count =
        u32::try_from(count).map_err(|_| DueSetFingerprintError::CountOverflow { count })?;
    NonZeroU32::new(count).ok_or(DueSetFingerprintError::Empty)
}

fn due_set_fingerprint_bytes(
    fired: SimMoment,
    resulting_frontier: SimMoment,
    due_count: NonZeroU32,
    due_keys: &[SchedulerKey],
) -> CanonicalBytes {
    debug_assert_eq!(usize::try_from(due_count.get()).ok(), Some(due_keys.len()));
    let mut writer = CanonicalWriter::new(DUE_SET_FINGERPRINT_DOMAIN);
    writer.write_u16(DUE_SET_FINGERPRINT_SCHEMA_VERSION);
    write_moment(&mut writer, fired);
    write_moment(&mut writer, resulting_frontier);
    writer.write_u32(due_count.get());
    for key in due_keys {
        write_scheduler_key(&mut writer, *key);
    }
    writer.finish()
}

fn attempt_step_bytes(
    attempt: RunAttemptId,
    expected: AuthorityCursor,
    operation: ReservedOperationFingerprint,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(ATTEMPT_STEP_DOMAIN);
    writer.write_u16(ATTEMPT_STEP_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, attempt.as_bytes());
    write_owned_bytes(&mut writer, expected.canonical_bytes().as_bytes());
    write_fixed_bytes(&mut writer, operation.as_bytes());
    writer.finish()
}

fn write_management_operation(writer: &mut CanonicalWriter, operation: SessionManagement) {
    match operation {
        SessionManagement::Pause => writer.write_discriminant(0),
        SessionManagement::Resume => writer.write_discriminant(1),
        SessionManagement::Retire(retirement) => {
            writer.write_discriminant(2);
            match retirement {
                LedgerRetirement::InputThrough(target) => {
                    writer.write_discriminant(0);
                    writer.write_u64(target.get());
                }
                LedgerRetirement::ManagementThrough(target) => {
                    writer.write_discriminant(1);
                    writer.write_u64(target.get());
                }
                LedgerRetirement::CommandThrough { source, command } => {
                    writer.write_discriminant(2);
                    write_fixed_bytes(writer, source.as_bytes());
                    writer.write_u64(command.get());
                }
            }
        }
        SessionManagement::SealAdmissionThrough(frontier) => {
            writer.write_discriminant(3);
            write_moment(writer, frontier);
        }
        SessionManagement::Quarantine => writer.write_discriminant(4),
        SessionManagement::Fail => writer.write_discriminant(5),
        SessionManagement::ResolveActionEvaluation {
            invocation,
            disposition,
        } => {
            writer.write_discriminant(6);
            write_fixed_bytes(writer, invocation.as_bytes());
            writer.write_discriminant(disposition.canonical_tag());
        }
    }
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

fn write_scheduler_key(writer: &mut CanonicalWriter, key: SchedulerKey) {
    write_moment(writer, key.moment());
    writer.write_discriminant(key.lane().canonical_tag());
    writer.write_u64(key.sequence().get());
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

fn write_owned_bytes(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("owned canonical bytes must fit the canonical protocol");
    }
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8; 32]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::fmt;

    use world_core::{Microstep, SimTime};
    use world_model::{AcceptedState, AgencyState, DomainState, EpistemicState, SocialState};

    use crate::authority::CapturedInputRecordId;
    use crate::control::test_support;
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, ExternalInputNamespaceId, InitialStateRootV1,
        ResolvedExecutionClosureManifestV1, RootSeed, TerminationContractV1,
    };
    use crate::kernel::{InputId, ManageRequest, ManagementRequestId};
    use crate::scheduler::{
        PreparedScheduledCommand, ScheduledWork, SchedulerInsertion, SchedulerProducerOrdinal,
        SchedulerState,
    };
    use crate::session::SessionMode;

    use super::super::binding::{AttemptAuthorityDomainId, AttemptCreation, AttemptKey};
    use super::super::disposition::AttemptDisposition;
    use super::*;

    fn valid<T, E: fmt::Display>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("reservation fixture must be valid: {error}"),
        }
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn closure(seed_byte: u8) -> ResolvedExecutionClosureManifestV1 {
        let semantics = valid(ExecutionSemanticsManifestV1::new(
            test_support::definitions(),
            crate::execution::fixture_lifecycle_profiles(),
            valid(ExecutionConfigArtifactV3::inline(64, 32, 16)),
            Vec::new(),
        ));
        let root = valid(InitialStateRootV1::origin(
            SessionMode::Running,
            SimMoment::ORIGIN,
            SimMoment::ORIGIN,
            AcceptedState::new(
                valid(DomainState::new(Vec::new(), Vec::new(), Vec::new())),
                EpistemicState::empty(),
                SocialState::empty(),
                AgencyState::empty(),
            ),
            Vec::new(),
        ));
        let specification = CanonicalExecutionSpecV1::new(
            &root,
            &semantics,
            RootSeed::from_bytes([seed_byte; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ))
    }

    fn attempt(domain_byte: u8, key_byte: u8, seed_byte: u8) -> (AttemptBinding, AuthorityCursor) {
        let closure = closure(seed_byte);
        let creation = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([domain_byte; 32]),
            AttemptKey::from_bytes([key_byte; 32]),
            &closure,
        );
        (creation.binding(), closure.root_cursor())
    }

    fn admit_descriptor() -> ReservedOperationDescriptor {
        ReservedOperationDescriptor::admit_command(&AdmitRequest::new(
            InputId::new(7),
            moment(5, 3),
            test_support::command(0x31, 11),
        ))
    }

    fn command_work(
        input: u64,
        due: SimMoment,
        command_byte: u8,
        captured_byte: u8,
    ) -> ScheduledWork {
        let request = AdmitRequest::new(
            InputId::new(input),
            due,
            test_support::command(command_byte, input),
        );
        ScheduledWork::command(
            PreparedScheduledCommand::prepare(
                ExternalInputNamespaceId::from_bytes([0x72; 32]),
                &request,
            )
            .materialize(CapturedInputRecordId::from_bytes([captured_byte; 32])),
        )
    }

    fn command_key(due: SimMoment) -> SchedulerKey {
        let plan = SchedulerState::empty()
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                command_work(80, due, 0x70, 0x71),
            )])
            .unwrap_or_else(|error| {
                panic!("command publication must fit an empty scheduler: {error:?}")
            });
        plan.entries()[0].0
    }

    fn scheduler_after_one_install() -> SchedulerState {
        let mut scheduler = SchedulerState::empty();
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                command_work(81, moment(1, 0), 0x71, 0x73),
            )])
            .unwrap_or_else(|error| {
                panic!("first command publication must fit an empty scheduler: {error:?}")
            });
        scheduler
            .install_batch_exact(plan)
            .unwrap_or_else(|error| panic!("planned command publication must install: {error:?}"));
        scheduler
    }

    fn command_key_after_one_install(due: SimMoment) -> SchedulerKey {
        let scheduler = scheduler_after_one_install();
        let plan = scheduler
            .plan_batch(vec![SchedulerInsertion::new(
                SchedulerProducerOrdinal::new(0),
                command_work(82, due, 0x74, 0x75),
            )])
            .unwrap_or_else(|error| {
                panic!("second command publication must fit the scheduler: {error:?}")
            });
        plan.entries()[0].0
    }

    fn command_fire_descriptor() -> ReservedOperationDescriptor {
        let fired = moment(5, 3);
        let due_keys = [command_key(fired), command_key_after_one_install(fired)];
        valid(ReservedOperationDescriptor::fire(
            fired,
            moment(5, 4),
            &due_keys,
        ))
    }

    fn manage_descriptor() -> ReservedOperationDescriptor {
        ReservedOperationDescriptor::manage(ManageRequest::new(
            ManagementRequestId::new(9),
            SessionManagement::Pause,
        ))
    }

    fn kernel_safety_descriptor() -> ReservedOperationDescriptor {
        let due = moment(5, 3);
        let due_keys = [command_key(due), command_key_after_one_install(due)];
        let cause = valid(KernelSafetyCause::due_work_population_exceeded(
            NonZeroU32::new(1).unwrap_or_else(|| unreachable!("fixture limit is nonzero")),
            moment(5, 4),
            &due_keys,
        ));
        ReservedOperationDescriptor::kernel_safety(cause)
    }

    fn capture_descriptor() -> ReservedOperationDescriptor {
        ReservedOperationDescriptor::AdmitActionEvaluation {
            capture: ActionEvaluationCaptureId::new(13),
            fingerprint: ActionEvaluationCaptureFingerprint::from_bytes([0x81; 32]),
            invocation: ActionEvaluationInvocationId::from_bytes([0x82; 32]),
            effective: moment(7, 2),
        }
    }

    #[test]
    fn descriptor_variants_are_separate_and_derive_distinct_fingerprints() {
        let descriptors = [
            admit_descriptor(),
            command_fire_descriptor(),
            manage_descriptor(),
            kernel_safety_descriptor(),
            capture_descriptor(),
        ];
        let fingerprints = descriptors.map(ReservedOperationFingerprint::derive);

        for (index, fingerprint) in fingerprints.iter().enumerate() {
            assert!(!fingerprints[index + 1..].contains(fingerprint));
        }
    }

    #[test]
    fn admit_fingerprint_commits_every_field() {
        let base = admit_descriptor();
        let ReservedOperationDescriptor::AdmitCommand {
            id,
            fingerprint,
            effective,
        } = base
        else {
            panic!("fixture must be an Admit descriptor");
        };
        let other_request = AdmitRequest::new(id, effective, test_support::command(0x32, 11));
        let base_fingerprint = ReservedOperationFingerprint::derive(base);

        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::AdmitCommand {
                id: InputId::new(id.get() + 1),
                fingerprint,
                effective,
            })
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::AdmitCommand {
                id,
                fingerprint: other_request.fingerprint(),
                effective,
            })
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::AdmitCommand {
                id,
                fingerprint,
                effective: moment(effective.time().ticks() + 1, effective.microstep().get()),
            })
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::AdmitCommand {
                id,
                fingerprint,
                effective: moment(effective.time().ticks(), effective.microstep().get() + 1),
            })
        );
    }

    #[test]
    fn fire_fingerprint_commits_the_complete_due_selector() {
        let base = command_fire_descriptor();
        let ReservedOperationDescriptor::Fire {
            fired,
            resulting_frontier,
            due_count,
            due_set_fingerprint,
        } = base
        else {
            panic!("fixture must be a Fire descriptor");
        };
        let base_fingerprint = ReservedOperationFingerprint::derive(base);
        let first = command_key(fired);
        let second = command_key_after_one_install(fired);

        assert_eq!(due_count.get(), 2);
        assert_eq!(
            due_set_fingerprint,
            DueSetFingerprint::derive(fired, resulting_frontier, due_count, &[first, second])
        );

        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(valid(ReservedOperationDescriptor::fire(
                moment(fired.time().ticks() + 1, fired.microstep().get()),
                resulting_frontier,
                &[first, second],
            )))
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(valid(ReservedOperationDescriptor::fire(
                fired,
                moment(
                    resulting_frontier.time().ticks(),
                    resulting_frontier.microstep().get() + 1,
                ),
                &[first, second],
            )))
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(valid(ReservedOperationDescriptor::fire(
                fired,
                resulting_frontier,
                &[first],
            )))
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(valid(ReservedOperationDescriptor::fire(
                fired,
                resulting_frontier,
                &[second, first],
            )))
        );
    }

    #[test]
    fn fire_due_set_count_is_checked_before_hashing() {
        assert_eq!(
            ReservedOperationDescriptor::fire(moment(5, 3), moment(5, 4), &[]),
            Err(DueSetFingerprintError::Empty)
        );

        if let Some(count) = usize::try_from(u32::MAX)
            .ok()
            .and_then(|maximum| maximum.checked_add(1))
        {
            assert_eq!(
                checked_due_count(count),
                Err(DueSetFingerprintError::CountOverflow { count })
            );
        }
    }

    #[test]
    fn manage_fingerprint_commits_every_field() {
        let base = manage_descriptor();
        let ReservedOperationDescriptor::Manage {
            id,
            fingerprint,
            operation,
        } = base
        else {
            panic!("fixture must be a Manage descriptor");
        };
        let resumed = ManageRequest::new(id, SessionManagement::Resume);
        let base_fingerprint = ReservedOperationFingerprint::derive(base);

        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::Manage {
                id: ManagementRequestId::new(id.get() + 1),
                fingerprint,
                operation,
            })
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::Manage {
                id,
                fingerprint: resumed.fingerprint(),
                operation,
            })
        );
        assert_ne!(
            base_fingerprint,
            ReservedOperationFingerprint::derive(ReservedOperationDescriptor::Manage {
                id,
                fingerprint,
                operation: SessionManagement::Resume,
            })
        );
    }

    #[test]
    fn kernel_safety_fingerprint_commits_the_typed_cause() {
        let quarantined = kernel_safety_descriptor();
        let due = moment(5, 3);
        let due_keys = [command_key(due), command_key_after_one_install(due)];
        let paused = ReservedOperationDescriptor::kernel_safety(valid(
            KernelSafetyCause::same_time_wave_exhausted(
                NonZeroU32::new(1).unwrap_or_else(|| unreachable!("fixture limit is nonzero")),
                2,
                moment(5, 4),
                &due_keys,
            ),
        ));

        assert_ne!(
            ReservedOperationFingerprint::derive(quarantined),
            ReservedOperationFingerprint::derive(paused)
        );
        assert_ne!(quarantined.canonical_bytes(), paused.canonical_bytes());
    }

    #[test]
    fn capture_fingerprint_commits_its_distinct_namespace_and_complete_body() {
        let base = capture_descriptor();
        let ReservedOperationDescriptor::AdmitActionEvaluation {
            capture,
            fingerprint,
            invocation,
            effective,
        } = base
        else {
            panic!("fixture must be an action-evaluation capture descriptor");
        };
        let base_fingerprint = ReservedOperationFingerprint::derive(base);

        for changed in [
            ReservedOperationDescriptor::AdmitActionEvaluation {
                capture: ActionEvaluationCaptureId::new(capture.get() + 1),
                fingerprint,
                invocation,
                effective,
            },
            ReservedOperationDescriptor::AdmitActionEvaluation {
                capture,
                fingerprint: ActionEvaluationCaptureFingerprint::from_bytes([0x83; 32]),
                invocation,
                effective,
            },
            ReservedOperationDescriptor::AdmitActionEvaluation {
                capture,
                fingerprint,
                invocation: ActionEvaluationInvocationId::from_bytes([0x84; 32]),
                effective,
            },
            ReservedOperationDescriptor::AdmitActionEvaluation {
                capture,
                fingerprint,
                invocation,
                effective: moment(7, 3),
            },
        ] {
            assert_ne!(
                base_fingerprint,
                ReservedOperationFingerprint::derive(changed)
            );
        }
    }

    #[test]
    fn step_identity_commits_attempt_cursor_and_operation() {
        let (binding, expected) = attempt(0x11, 0x21, 0x61);
        let (other_binding, _) = attempt(0x11, 0x22, 0x61);
        let (_, other_expected) = attempt(0x11, 0x21, 0x62);
        let operation = ReservedOperationFingerprint::derive(command_fire_descriptor());
        let step = AttemptStepId::derive(binding.attempt(), expected, operation);

        assert_ne!(
            step,
            AttemptStepId::derive(other_binding.attempt(), expected, operation)
        );
        assert_ne!(
            step,
            AttemptStepId::derive(binding.attempt(), other_expected, operation)
        );
        assert_ne!(
            step,
            AttemptStepId::derive(
                binding.attempt(),
                expected,
                ReservedOperationFingerprint::derive(manage_descriptor()),
            )
        );
    }

    #[test]
    fn reservation_derives_all_evidence_and_starts_without_disposition() {
        let (binding, expected) = attempt(0x11, 0x21, 0x61);
        let operation = command_fire_descriptor();
        let reservation =
            StepReservation::new(binding, ReservationGrant::FIRST, expected, operation);
        let fingerprint = ReservedOperationFingerprint::derive(operation);

        assert_eq!(reservation.binding(), binding);
        assert_eq!(reservation.expected(), expected);
        assert_eq!(reservation.operation(), operation);
        assert_eq!(reservation.operation_fingerprint(), fingerprint);
        assert_eq!(
            reservation.step(),
            AttemptStepId::derive(binding.attempt(), expected, fingerprint)
        );
        assert_eq!(reservation.disposition(), None);
    }

    #[test]
    fn reservation_accepts_each_failure_disposition() {
        let failures = [
            AttemptDisposition::HostBudgetExceeded,
            AttemptDisposition::ExternalFailure,
            AttemptDisposition::EngineFailure,
        ];

        for failure in failures {
            let (binding, expected) = attempt(0x11, 0x21, 0x61);
            let mut reservation = StepReservation::new(
                binding,
                ReservationGrant::FIRST,
                expected,
                command_fire_descriptor(),
            );
            let id = failure.id();
            reservation
                .attach_failure(id)
                .unwrap_or_else(|error| panic!("failure evidence must attach: {error:?}"));
            assert_eq!(reservation.disposition(), Some(id));
            assert_eq!(id, failure.id());
        }
    }

    #[test]
    fn disposition_attachment_requires_a_fire_reservation() {
        let (binding, expected) = attempt(0x11, 0x21, 0x61);

        for operation in [
            admit_descriptor(),
            manage_descriptor(),
            kernel_safety_descriptor(),
        ] {
            let mut reservation =
                StepReservation::new(binding, ReservationGrant::FIRST, expected, operation);
            let disposition = AttemptDisposition::EngineFailure.id();
            assert_eq!(
                reservation.attach_failure(disposition),
                Err(ReservationFailureAttachError::NotFire {
                    operation: Box::new(operation),
                })
            );
            assert_eq!(reservation.disposition(), None);
        }
    }

    #[test]
    fn failure_attachment_is_one_way() {
        let (binding, expected) = attempt(0x11, 0x21, 0x61);
        let mut attached = StepReservation::new(
            binding,
            ReservationGrant::FIRST,
            expected,
            command_fire_descriptor(),
        );
        let existing = AttemptDisposition::EngineFailure.id();
        attached
            .attach_failure(existing)
            .unwrap_or_else(|error| panic!("first failure evidence must attach: {error:?}"));
        let second = AttemptDisposition::ExternalFailure.id();
        assert_eq!(
            attached.attach_failure(second),
            Err(ReservationFailureAttachError::AlreadyAttached { existing })
        );
        assert_eq!(attached.disposition(), Some(existing));
    }

    #[test]
    fn canonical_values_have_frozen_vectors() {
        let (binding, expected) = attempt(0x11, 0x21, 0x61);
        let admit = admit_descriptor();
        let command_fire = command_fire_descriptor();
        let manage = manage_descriptor();
        let admit_fingerprint = ReservedOperationFingerprint::derive(admit);
        let command_fire_fingerprint = ReservedOperationFingerprint::derive(command_fire);
        let manage_fingerprint = ReservedOperationFingerprint::derive(manage);
        let step = AttemptStepId::derive(binding.attempt(), expected, command_fire_fingerprint);
        let ReservedOperationDescriptor::Fire {
            fired,
            resulting_frontier,
            due_count,
            due_set_fingerprint,
        } = command_fire
        else {
            panic!("fixture must be a Fire descriptor");
        };
        let due_keys = [command_key(fired), command_key_after_one_install(fired)];

        assert_eq!(
            hex(admit.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001572657365727665642d6f70657261",
                "74696f6e2d7632000200000000000000000000000700000000000000201536f75f7185a60ba0edce",
                "0e709a130e666f82872cbac7167e0a719e7adf9c4e00000000000000050000000000000003",
            )
        );
        assert_eq!(
            admit_fingerprint.to_string(),
            "d0bb589001dfa969e9892b1492b25dab632cea46911d54d07f2692a2287ef442"
        );
        assert_eq!(
            hex(
                due_set_fingerprint_bytes(fired, resulting_frontier, due_count, &due_keys)
                    .as_bytes()
            ),
            concat!(
                "776f726c642d63616e6f6e6963616c2d763100000000000000166475652d7365742d66696e676572",
                "7072696e742d76320002000000000000000500000000000000030000000000000005000000000000",
                "00040000000200000000000000050000000000000003000000000000000000000000000000000000",
                "00050000000000000003000000000000000000000001",
            )
        );
        assert_eq!(
            due_set_fingerprint.to_string(),
            "7747cad9a38029d2c343a59e73c5d7415c4328872b59a785ffc2a0e3d69e7ff0"
        );
        assert_eq!(
            hex(command_fire.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001572657365727665642d6f7065726174696f6e2d763200020000000100000000000000050000000000000003000000000000000500000000000000040000000200000000000000207747cad9a38029d2c343a59e73c5d7415c4328872b59a785ffc2a0e3d69e7ff0"
        );
        assert_eq!(
            command_fire_fingerprint.to_string(),
            "40a49f60aa4ecd3412b8d70b88662410b5d267648a468f6043b845bf4222eb8a"
        );
        assert_eq!(
            hex(manage.canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001572657365727665642d6f70657261",
                "74696f6e2d763200020000000200000000000000090000000000000020a12c780c5033e15d75cfd2",
                "7e28cd9c00a3ed4e31fc5c6bad690f751535a8ee7b00000000",
            )
        );
        assert_eq!(
            manage_fingerprint.to_string(),
            "0bad04d3683ab3cb044c164d99a438edc44f1d40ae5143b0e44f856618a28e4c"
        );
        assert_eq!(
            hex(
                attempt_step_bytes(binding.attempt(), expected, command_fire_fingerprint)
                    .as_bytes()
            ),
            "776f726c642d63616e6f6e6963616c2d7631000000000000000c617474656d70742d73746570000100000000000000205b04ee472d1e969012759502c04d118accbc9d4b0579e089900bc8c610ca667600000000000000d3776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d637572736f722d763100010000000000000020b11dbda2fa2028edbcd0b4d91d3f44e515c5108300d1b890d19b4dbdb91bb48700000000000000202a68d1d8125de8bf0943b07745f316eb6ee7a5a91182b7c62530a45e311607a6000000000000000000000020a69ccc2caf60d5c3e776408d80491d5f885006191d59ff4a31dbaaccfc9a102600000000000000204d8475f8965ba58445250fe767dc55c4f8210550deaa99a4c9bfe788af1bca38000000000000002040a49f60aa4ecd3412b8d70b88662410b5d267648a468f6043b845bf4222eb8a"
        );
        assert_eq!(
            step.to_string(),
            "d756c155485c2481fdcb3b2b1871075d4e4013bb21cb850b53e90c3ac8cf3652"
        );
        assert_eq!(AttemptStepId::from_bytes(*step.as_bytes()), step);
    }

    #[test]
    fn action_evaluation_capture_has_a_frozen_vector() {
        assert_eq!(
            hex(capture_descriptor().canonical_bytes().as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d7631000000000000001572657365727665642d6f7065726174696f6e2d763200",
                "0200000004000000000000000d0000000000000020818181818181818181818181818181818181818181818181818181",
                "818181818100000000000000208282828282828282828282828282828282828282828282828282828282828282000000",
                "00000000070000000000000002",
            )
        );
    }

    fn hex(bytes: &[u8]) -> String {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let mut encoded = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
            encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        encoded
    }
}
