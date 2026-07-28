use core::fmt;
use core::num::NonZeroU32;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, SimMoment};

use crate::attempt::{
    AttemptAuthorityDomainId, AttemptStepId, DueSetFingerprint, DueSetFingerprintError,
    ReservationGrant, RunAttemptId,
};
use crate::authority::{AuthorityCursor, AuthorityRecordId};
use crate::execution::{ExecutionConfigArtifactV3, ExecutionSpecId};
use crate::scheduler::{SchedulerKey, SchedulerLaneV2, strictly_later_moment};

use super::PreparedFire;

/// Canonical schema of a deterministic kernel-safety cause.
pub const KERNEL_SAFETY_CAUSE_SCHEMA_VERSION: u16 = 2;

const KERNEL_SAFETY_CAUSE_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("kernel-safety-cause-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("kernel safety cause domain must be valid"),
    };

const TRIGGER_SAMPLE_CAPACITY: usize = 4;

/// Resulting session health selected by one deterministic safety cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KernelSafetyDisposition {
    /// Suspend ordinary execution while retaining the unresolved due set.
    Paused,
    /// Isolate a due set that deterministically exceeds a population limit.
    Quarantined,
    /// Enter terminal failure because simulation time cannot advance.
    Failed,
}

impl KernelSafetyDisposition {
    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::Paused => 0,
            Self::Quarantined => 1,
            Self::Failed => 2,
        }
    }
}

/// Stable scheduler lane retained in bounded safety diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KernelSafetyTriggerLane {
    /// Checked command delivery.
    Command,
    /// Post-commit reaction dispatch.
    PostCommit,
    /// Runtime-owned time-bearing process wake.
    Process,
    /// Deterministic lifecycle delivery or continuation.
    Lifecycle,
    /// Open action opportunity evaluation.
    ActionReady,
    /// Captured action-result interpretation or runtime fallback.
    ActionEvaluation,
    /// Outcome-neutral continuation after an action attempt.
    AttemptResolved,
}

impl KernelSafetyTriggerLane {
    const fn from_scheduler(lane: SchedulerLaneV2) -> Self {
        match lane {
            SchedulerLaneV2::Command => Self::Command,
            SchedulerLaneV2::PostCommit => Self::PostCommit,
            SchedulerLaneV2::Process => Self::Process,
            SchedulerLaneV2::Lifecycle => Self::Lifecycle,
            SchedulerLaneV2::ActionReady => Self::ActionReady,
            SchedulerLaneV2::ActionEvaluation => Self::ActionEvaluation,
            SchedulerLaneV2::AttemptResolved => Self::AttemptResolved,
        }
    }

    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::Command => 0,
            Self::PostCommit => 1,
            Self::Process => 2,
            Self::Lifecycle => 3,
            Self::ActionReady => 4,
            Self::ActionEvaluation => 5,
            Self::AttemptResolved => 6,
        }
    }
}

/// One bounded diagnostic coordinate from an unresolved due set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KernelSafetyTriggerCoordinate {
    moment: SimMoment,
    lane: KernelSafetyTriggerLane,
    sequence: u64,
}

impl KernelSafetyTriggerCoordinate {
    const fn from_scheduler(key: SchedulerKey) -> Self {
        Self {
            moment: key.moment(),
            lane: KernelSafetyTriggerLane::from_scheduler(key.lane()),
            sequence: key.sequence().get(),
        }
    }

    /// Returns the unresolved delivery moment.
    #[must_use]
    pub const fn moment(self) -> SimMoment {
        self.moment
    }

    /// Returns the stable work-family lane.
    #[must_use]
    pub const fn lane(self) -> KernelSafetyTriggerLane {
        self.lane
    }

    /// Returns the scheduler-owned insertion sequence.
    #[must_use]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
}

/// Bounded diagnostic prefix of the complete unresolved due set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelSafetyTriggerSample {
    coordinates: [Option<KernelSafetyTriggerCoordinate>; TRIGGER_SAMPLE_CAPACITY],
    len: u8,
}

impl KernelSafetyTriggerSample {
    fn from_keys(keys: &[SchedulerKey]) -> Self {
        let mut coordinates = [None; TRIGGER_SAMPLE_CAPACITY];
        let len = keys.len().min(TRIGGER_SAMPLE_CAPACITY);
        for (slot, key) in coordinates.iter_mut().zip(keys.iter()).take(len) {
            *slot = Some(KernelSafetyTriggerCoordinate::from_scheduler(*key));
        }
        Self {
            coordinates,
            len: u8::try_from(len).unwrap_or(TRIGGER_SAMPLE_CAPACITY as u8),
        }
    }

    /// Returns the number of retained diagnostic coordinates.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len as usize
    }

    /// Returns whether the bounded sample is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Returns one retained coordinate by diagnostic position.
    #[must_use]
    pub const fn get(self, position: usize) -> Option<KernelSafetyTriggerCoordinate> {
        if position < self.len as usize {
            self.coordinates[position]
        } else {
            None
        }
    }
}

/// Complete fixed-width identity plus bounded diagnostics for an unresolved due set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelSafetyDueSetEvidence {
    due: SimMoment,
    preserved_frontier: SimMoment,
    due_count: NonZeroU32,
    due_set_fingerprint: DueSetFingerprint,
    sample: KernelSafetyTriggerSample,
}

impl KernelSafetyDueSetEvidence {
    fn checked(
        preserved_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<Self, KernelSafetyCauseBuildError> {
        let Some(first) = due_keys.first().copied() else {
            return Err(DueSetFingerprintError::Empty.into());
        };
        let due = first.moment();

        for key in due_keys {
            if key.moment() != due {
                return Err(KernelSafetyCauseBuildError::MixedDueMoment {
                    expected: due,
                    supplied: key.moment(),
                });
            }
        }
        for pair in due_keys.windows(2) {
            if pair[0] >= pair[1] {
                return Err(KernelSafetyCauseBuildError::NonCanonicalDueSet {
                    previous: pair[0],
                    supplied: pair[1],
                });
            }
        }

        let (due_count, due_set_fingerprint) =
            DueSetFingerprint::derive_checked(due, preserved_frontier, due_keys)?;
        Ok(Self {
            due,
            preserved_frontier,
            due_count,
            due_set_fingerprint,
            sample: KernelSafetyTriggerSample::from_keys(due_keys),
        })
    }

    /// Returns the exact unresolved scheduler moment.
    #[must_use]
    pub const fn due(self) -> SimMoment {
        self.due
    }

    /// Returns the unchanged admission frontier.
    #[must_use]
    pub const fn preserved_frontier(self) -> SimMoment {
        self.preserved_frontier
    }

    /// Returns the complete unresolved trigger count.
    #[must_use]
    pub const fn due_count(self) -> NonZeroU32 {
        self.due_count
    }

    /// Returns the fingerprint of every ordered scheduler key in the due set.
    #[must_use]
    pub const fn due_set_fingerprint(&self) -> &[u8; 32] {
        self.due_set_fingerprint.as_bytes()
    }

    /// Returns the bounded diagnostic prefix of unresolved trigger coordinates.
    #[must_use]
    pub const fn sample(self) -> KernelSafetyTriggerSample {
        self.sample
    }
}

/// Deterministic reason ordinary Fire could not be prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelSafetyCause {
    /// The complete due set exceeds the configured per-moment work bound.
    DueWorkPopulationExceeded {
        /// Configured nonzero bound.
        limit: NonZeroU32,
        /// Exact observed work count.
        observed: u64,
        /// Identity and bounded diagnostics of the preserved due set.
        evidence: KernelSafetyDueSetEvidence,
    },
    /// The commands requiring evaluation exceed the configured candidate bound.
    EvaluableCommandPopulationExceeded {
        /// Configured nonzero bound.
        limit: NonZeroU32,
        /// Exact observed evaluable-command count.
        observed: u64,
        /// Identity and bounded diagnostics of the preserved due set.
        evidence: KernelSafetyDueSetEvidence,
    },
    /// The next Fire would exceed the configured same-time wave tranche.
    SameTimeWaveExhausted {
        /// Configured nonzero bound.
        limit: NonZeroU32,
        /// One-based wave number that was refused.
        attempted_wave: u64,
        /// Identity and bounded diagnostics of the preserved due set.
        evidence: KernelSafetyDueSetEvidence,
    },
    /// No scheduler moment exists strictly after the unresolved terminal moment.
    TerminalClockExhausted {
        /// Identity and bounded diagnostics of the preserved due set.
        evidence: KernelSafetyDueSetEvidence,
    },
}

impl KernelSafetyCause {
    pub(crate) fn due_work_population_exceeded(
        limit: NonZeroU32,
        preserved_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<Self, KernelSafetyCauseBuildError> {
        let evidence = KernelSafetyDueSetEvidence::checked(preserved_frontier, due_keys)?;
        let observed = u64::from(evidence.due_count().get());
        ensure_exceeded(limit, observed)?;
        Ok(Self::DueWorkPopulationExceeded {
            limit,
            observed,
            evidence,
        })
    }

    pub(crate) fn evaluable_command_population_exceeded(
        limit: NonZeroU32,
        observed: u64,
        preserved_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<Self, KernelSafetyCauseBuildError> {
        let evidence = KernelSafetyDueSetEvidence::checked(preserved_frontier, due_keys)?;
        if observed > u64::from(evidence.due_count().get()) {
            return Err(
                KernelSafetyCauseBuildError::EvaluablePopulationExceedsDueSet {
                    evaluable: observed,
                    due: evidence.due_count(),
                },
            );
        }
        ensure_exceeded(limit, observed)?;
        Ok(Self::EvaluableCommandPopulationExceeded {
            limit,
            observed,
            evidence,
        })
    }

    pub(crate) fn same_time_wave_exhausted(
        limit: NonZeroU32,
        attempted_wave: u64,
        preserved_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<Self, KernelSafetyCauseBuildError> {
        let evidence = KernelSafetyDueSetEvidence::checked(preserved_frontier, due_keys)?;
        ensure_exceeded(limit, attempted_wave)?;
        Ok(Self::SameTimeWaveExhausted {
            limit,
            attempted_wave,
            evidence,
        })
    }

    pub(crate) fn terminal_clock_exhausted(
        preserved_frontier: SimMoment,
        due_keys: &[SchedulerKey],
    ) -> Result<Self, KernelSafetyCauseBuildError> {
        let evidence = KernelSafetyDueSetEvidence::checked(preserved_frontier, due_keys)?;
        if strictly_later_moment(evidence.due()).is_ok() {
            return Err(KernelSafetyCauseBuildError::ClockCanAdvance {
                due: evidence.due(),
            });
        }
        Ok(Self::TerminalClockExhausted { evidence })
    }

    /// Returns the configured health transition for this cause family.
    #[must_use]
    pub const fn disposition(self) -> KernelSafetyDisposition {
        match self {
            Self::DueWorkPopulationExceeded { .. }
            | Self::EvaluableCommandPopulationExceeded { .. } => {
                KernelSafetyDisposition::Quarantined
            }
            Self::SameTimeWaveExhausted { .. } => KernelSafetyDisposition::Paused,
            Self::TerminalClockExhausted { .. } => KernelSafetyDisposition::Failed,
        }
    }

    /// Returns the preserved due-set evidence shared by every cause family.
    #[must_use]
    pub const fn evidence(self) -> KernelSafetyDueSetEvidence {
        match self {
            Self::DueWorkPopulationExceeded { evidence, .. }
            | Self::EvaluableCommandPopulationExceeded { evidence, .. }
            | Self::SameTimeWaveExhausted { evidence, .. }
            | Self::TerminalClockExhausted { evidence } => evidence,
        }
    }

    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::DueWorkPopulationExceeded { .. } => 0,
            Self::EvaluableCommandPopulationExceeded { .. } => 1,
            Self::SameTimeWaveExhausted { .. } => 2,
            Self::TerminalClockExhausted { .. } => 3,
        }
    }

    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        kernel_safety_cause_bytes(self)
    }

    pub(crate) const fn permits_resume(self) -> bool {
        matches!(self, Self::SameTimeWaveExhausted { .. })
    }
}

/// Why deterministic safety evidence could not be constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KernelSafetyCauseBuildError {
    DueSet(DueSetFingerprintError),
    MixedDueMoment {
        expected: SimMoment,
        supplied: SimMoment,
    },
    NonCanonicalDueSet {
        previous: SchedulerKey,
        supplied: SchedulerKey,
    },
    LimitNotExceeded {
        limit: NonZeroU32,
        observed: u64,
    },
    EvaluablePopulationExceedsDueSet {
        evaluable: u64,
        due: NonZeroU32,
    },
    ClockCanAdvance {
        due: SimMoment,
    },
}

impl From<DueSetFingerprintError> for KernelSafetyCauseBuildError {
    fn from(error: DueSetFingerprintError) -> Self {
        Self::DueSet(error)
    }
}

impl fmt::Display for KernelSafetyCauseBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DueSet(error) => error.fmt(formatter),
            Self::MixedDueMoment { expected, supplied } => write!(
                formatter,
                "kernel safety due set mixes moments {expected:?} and {supplied:?}"
            ),
            Self::NonCanonicalDueSet { previous, supplied } => write!(
                formatter,
                "kernel safety due keys are not strictly ordered: {previous:?}, {supplied:?}"
            ),
            Self::LimitNotExceeded { limit, observed } => write!(
                formatter,
                "observed population {observed} does not exceed limit {limit}"
            ),
            Self::EvaluablePopulationExceedsDueSet { evaluable, due } => write!(
                formatter,
                "evaluable population {evaluable} exceeds due-set population {due}"
            ),
            Self::ClockCanAdvance { due } => {
                write!(formatter, "scheduler moment {due:?} is not terminal")
            }
        }
    }
}

impl std::error::Error for KernelSafetyCauseBuildError {}

/// Inspectable reason ordinary work remains blocked at the session head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelSafetyBlocker {
    cause: KernelSafetyCause,
}

impl KernelSafetyBlocker {
    pub(crate) const fn new(cause: KernelSafetyCause) -> Self {
        Self { cause }
    }

    /// Returns the exact deterministic blocking cause.
    #[must_use]
    pub const fn cause(self) -> KernelSafetyCause {
        self.cause
    }

    /// Returns the resulting session health disposition.
    #[must_use]
    pub const fn disposition(self) -> KernelSafetyDisposition {
        self.cause.disposition()
    }

    pub(crate) const fn permits_resume(self) -> bool {
        self.cause.permits_resume()
    }
}

/// Single-use authority capability for one deterministic safety publication.
///
/// Dropping this value performs no cleanup. The repository retains the
/// reservation until same-domain reconciliation.
pub struct PreparedKernelSafety {
    domain: AttemptAuthorityDomainId,
    attempt: RunAttemptId,
    execution: ExecutionSpecId,
    step: AttemptStepId,
    grant: ReservationGrant,
    expected: AuthorityCursor,
    cause: KernelSafetyCause,
}

impl PreparedKernelSafety {
    pub(crate) const fn new(
        domain: AttemptAuthorityDomainId,
        attempt: RunAttemptId,
        execution: ExecutionSpecId,
        step: AttemptStepId,
        grant: ReservationGrant,
        expected: AuthorityCursor,
        cause: KernelSafetyCause,
    ) -> Self {
        Self {
            domain,
            attempt,
            execution,
            step,
            grant,
            expected,
            cause,
        }
    }

    pub(crate) const fn domain(&self) -> AttemptAuthorityDomainId {
        self.domain
    }

    pub(crate) const fn attempt(&self) -> RunAttemptId {
        self.attempt
    }

    pub(crate) const fn execution(&self) -> ExecutionSpecId {
        self.execution
    }

    pub(crate) const fn step(&self) -> AttemptStepId {
        self.step
    }

    pub(crate) const fn grant(&self) -> ReservationGrant {
        self.grant
    }

    pub(crate) const fn expected(&self) -> AuthorityCursor {
        self.expected
    }

    /// Returns the exact typed safety cause carried by this capability.
    #[must_use]
    pub const fn cause(&self) -> KernelSafetyCause {
        self.cause
    }
}

/// Result of publishing one deterministic kernel-safety transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelSafetyOutcome {
    record: AuthorityRecordId,
    cursor: AuthorityCursor,
    cause: KernelSafetyCause,
}

impl KernelSafetyOutcome {
    pub(crate) const fn published(
        record: AuthorityRecordId,
        cursor: AuthorityCursor,
        cause: KernelSafetyCause,
    ) -> Self {
        Self {
            record,
            cursor,
            cause,
        }
    }

    /// Returns the authority record that captured the safety transition.
    #[must_use]
    pub const fn record(self) -> AuthorityRecordId {
        self.record
    }

    /// Returns the exact resulting authority cursor.
    #[must_use]
    pub const fn cursor(self) -> AuthorityCursor {
        self.cursor
    }

    /// Returns the exact deterministic safety cause.
    #[must_use]
    pub const fn cause(self) -> KernelSafetyCause {
        self.cause
    }

    /// Returns the resulting session health disposition.
    #[must_use]
    pub const fn disposition(self) -> KernelSafetyDisposition {
        self.cause.disposition()
    }
}

/// Result of serialized Fire preflight.
pub enum FirePreparation {
    /// The complete least-due moment is reserved for evaluation.
    Ready(PreparedFire),
    /// Ordinary work remains untouched and a safety publication is reserved.
    KernelSafety(PreparedKernelSafety),
}

pub(crate) fn select_kernel_safety_cause(
    config: ExecutionConfigArtifactV3,
    preserved_frontier: SimMoment,
    due_keys: &[SchedulerKey],
    evaluable_commands: u64,
    attempted_wave: u64,
) -> Result<Option<KernelSafetyCause>, KernelSafetyCauseBuildError> {
    let Some(due) = due_keys.first().map(|key| key.moment()) else {
        return Err(DueSetFingerprintError::Empty.into());
    };

    if strictly_later_moment(due).is_err() {
        return KernelSafetyCause::terminal_clock_exhausted(preserved_frontier, due_keys).map(Some);
    }

    let observed_work = u64::try_from(due_keys.len()).unwrap_or(u64::MAX);
    if observed_work > u64::from(config.maximum_work_per_moment().get()) {
        return KernelSafetyCause::due_work_population_exceeded(
            config.maximum_work_per_moment(),
            preserved_frontier,
            due_keys,
        )
        .map(Some);
    }

    if evaluable_commands > u64::from(config.maximum_evaluable_commands().get()) {
        return KernelSafetyCause::evaluable_command_population_exceeded(
            config.maximum_evaluable_commands(),
            evaluable_commands,
            preserved_frontier,
            due_keys,
        )
        .map(Some);
    }

    if attempted_wave > u64::from(config.maximum_same_time_waves().get()) {
        return KernelSafetyCause::same_time_wave_exhausted(
            config.maximum_same_time_waves(),
            attempted_wave,
            preserved_frontier,
            due_keys,
        )
        .map(Some);
    }

    Ok(None)
}

fn ensure_exceeded(limit: NonZeroU32, observed: u64) -> Result<(), KernelSafetyCauseBuildError> {
    if observed > u64::from(limit.get()) {
        Ok(())
    } else {
        Err(KernelSafetyCauseBuildError::LimitNotExceeded { limit, observed })
    }
}

fn kernel_safety_cause_bytes(cause: KernelSafetyCause) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(KERNEL_SAFETY_CAUSE_DOMAIN);
    writer.write_u16(KERNEL_SAFETY_CAUSE_SCHEMA_VERSION);
    writer.write_discriminant(cause.canonical_tag());
    writer.write_discriminant(cause.disposition().canonical_tag());
    match cause {
        KernelSafetyCause::DueWorkPopulationExceeded {
            limit,
            observed,
            evidence,
        }
        | KernelSafetyCause::EvaluableCommandPopulationExceeded {
            limit,
            observed,
            evidence,
        } => {
            writer.write_u32(limit.get());
            writer.write_u64(observed);
            write_due_set_evidence(&mut writer, evidence);
        }
        KernelSafetyCause::SameTimeWaveExhausted {
            limit,
            attempted_wave,
            evidence,
        } => {
            writer.write_u32(limit.get());
            writer.write_u64(attempted_wave);
            write_due_set_evidence(&mut writer, evidence);
        }
        KernelSafetyCause::TerminalClockExhausted { evidence } => {
            write_due_set_evidence(&mut writer, evidence);
        }
    }
    writer.finish()
}

fn write_due_set_evidence(writer: &mut CanonicalWriter, evidence: KernelSafetyDueSetEvidence) {
    write_moment(writer, evidence.due());
    write_moment(writer, evidence.preserved_frontier());
    writer.write_u32(evidence.due_count().get());
    write_fixed_bytes(writer, evidence.due_set_fingerprint());
    let sample = evidence.sample();
    writer.write_u32(sample.len() as u32);
    for position in 0..sample.len() {
        let coordinate = sample
            .get(position)
            .unwrap_or_else(|| unreachable!("sample length must cover every encoded coordinate"));
        write_moment(writer, coordinate.moment());
        writer.write_discriminant(coordinate.lane().canonical_tag());
        writer.write_u64(coordinate.sequence());
    }
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

#[cfg(test)]
mod tests {
    use core::{fmt::Debug, num::NonZeroU32};

    use world_core::{Microstep, SimTime};

    use crate::authority::CapturedInputRecordId;
    use crate::execution::ExternalInputNamespaceId;
    use crate::kernel::{AdmitRequest, InputId, fixtures};
    use crate::scheduler::{
        PreparedScheduledCommand, ScheduledWork, SchedulerInsertion, SchedulerProducerOrdinal,
        SchedulerState,
    };

    use super::*;

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn valid<T, E: Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("safety fixture must be valid: {error:?}"),
        }
    }

    fn nonzero(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap_or_else(|| panic!("safety fixture limit must be nonzero"))
    }

    fn present<T>(value: Option<T>) -> T {
        value.unwrap_or_else(|| panic!("safety fixture value must be present"))
    }

    fn due_keys(due: SimMoment, count: usize) -> Vec<SchedulerKey> {
        let insertions = (0..count)
            .map(|position| {
                let command_id = u64::try_from(position).unwrap_or(u64::MAX);
                let request = AdmitRequest::new(
                    InputId::new(command_id),
                    due,
                    fixtures::command(0x41, command_id),
                );
                let scheduled = PreparedScheduledCommand::prepare(
                    ExternalInputNamespaceId::from_bytes([0x51; 32]),
                    &request,
                )
                .materialize(CapturedInputRecordId::from_bytes([position as u8; 32]));
                SchedulerInsertion::new(
                    SchedulerProducerOrdinal::new(u32::try_from(position).unwrap_or(u32::MAX)),
                    ScheduledWork::command(scheduled),
                )
            })
            .collect();
        SchedulerState::empty()
            .plan_batch(insertions)
            .unwrap_or_else(|error| panic!("safety fixture must plan: {error:?}"))
            .entries()
            .iter()
            .map(|(key, _)| *key)
            .collect()
    }

    #[test]
    fn population_excess_is_quarantined_and_commits_the_complete_due_set() {
        let due = moment(7, 2);
        let frontier = moment(7, 3);
        let keys = due_keys(due, 6);
        let cause = valid(KernelSafetyCause::due_work_population_exceeded(
            nonzero(4),
            frontier,
            &keys,
        ));

        let KernelSafetyCause::DueWorkPopulationExceeded {
            limit,
            observed,
            evidence,
        } = cause
        else {
            panic!("constructor must retain its cause family");
        };
        assert_eq!(limit.get(), 4);
        assert_eq!(observed, 6);
        assert_eq!(cause.disposition(), KernelSafetyDisposition::Quarantined);
        assert_eq!(evidence.due(), due);
        assert_eq!(evidence.preserved_frontier(), frontier);
        assert_eq!(evidence.due_count().get(), 6);
        assert_eq!(evidence.sample().len(), TRIGGER_SAMPLE_CAPACITY);
        assert_eq!(present(evidence.sample().get(0)).moment(), due);

        let reordered = [keys[1], keys[0]];
        assert!(matches!(
            KernelSafetyCause::due_work_population_exceeded(nonzero(1), frontier, &reordered),
            Err(KernelSafetyCauseBuildError::NonCanonicalDueSet { .. })
        ));
    }

    #[test]
    fn evaluable_population_is_bounded_by_both_limit_and_due_set() {
        let keys = due_keys(moment(8, 1), 3);
        let frontier = moment(8, 2);

        assert!(matches!(
            KernelSafetyCause::evaluable_command_population_exceeded(
                nonzero(2),
                2,
                frontier,
                &keys
            ),
            Err(KernelSafetyCauseBuildError::LimitNotExceeded { .. })
        ));
        assert!(matches!(
            KernelSafetyCause::evaluable_command_population_exceeded(
                nonzero(2),
                4,
                frontier,
                &keys
            ),
            Err(KernelSafetyCauseBuildError::EvaluablePopulationExceedsDueSet { .. })
        ));
    }

    #[test]
    fn wave_exhaustion_is_the_only_resumable_safety_blocker() {
        let keys = due_keys(moment(9, 4), 1);
        let wave = valid(KernelSafetyCause::same_time_wave_exhausted(
            nonzero(3),
            4,
            moment(9, 5),
            &keys,
        ));
        let population = valid(KernelSafetyCause::due_work_population_exceeded(
            nonzero(1),
            moment(9, 5),
            &due_keys(moment(9, 4), 2),
        ));

        assert_eq!(wave.disposition(), KernelSafetyDisposition::Paused);
        assert!(KernelSafetyBlocker::new(wave).permits_resume());
        assert!(!KernelSafetyBlocker::new(population).permits_resume());
    }

    #[test]
    fn terminal_clock_exhaustion_is_failed_and_requires_a_terminal_due_moment() {
        let terminal = moment(u64::MAX, u64::MAX);
        let cause = valid(KernelSafetyCause::terminal_clock_exhausted(
            terminal,
            &due_keys(terminal, 1),
        ));
        assert_eq!(cause.disposition(), KernelSafetyDisposition::Failed);

        let nonterminal = moment(u64::MAX - 1, u64::MAX);
        assert_eq!(
            KernelSafetyCause::terminal_clock_exhausted(terminal, &due_keys(nonterminal, 1)),
            Err(KernelSafetyCauseBuildError::ClockCanAdvance { due: nonterminal })
        );
    }

    #[test]
    fn canonical_tags_are_frozen() {
        assert_eq!(KERNEL_SAFETY_CAUSE_SCHEMA_VERSION, 2);
        assert_eq!(
            KERNEL_SAFETY_CAUSE_DOMAIN.as_str(),
            "kernel-safety-cause-v2"
        );
        assert_eq!(KernelSafetyDisposition::Paused.canonical_tag(), 0);
        assert_eq!(KernelSafetyDisposition::Quarantined.canonical_tag(), 1);
        assert_eq!(KernelSafetyDisposition::Failed.canonical_tag(), 2);
        assert_eq!(KernelSafetyTriggerLane::Command.canonical_tag(), 0);
        assert_eq!(KernelSafetyTriggerLane::PostCommit.canonical_tag(), 1);
        assert_eq!(KernelSafetyTriggerLane::Process.canonical_tag(), 2);
        assert_eq!(KernelSafetyTriggerLane::Lifecycle.canonical_tag(), 3);
        assert_eq!(KernelSafetyTriggerLane::ActionReady.canonical_tag(), 4);
        assert_eq!(KernelSafetyTriggerLane::ActionEvaluation.canonical_tag(), 5);
        assert_eq!(KernelSafetyTriggerLane::AttemptResolved.canonical_tag(), 6);

        let frontier = moment(12, 2);
        let keys = due_keys(moment(12, 1), 2);
        let due = valid(KernelSafetyCause::due_work_population_exceeded(
            nonzero(1),
            frontier,
            &keys,
        ));
        let evaluable = valid(KernelSafetyCause::evaluable_command_population_exceeded(
            nonzero(1),
            2,
            frontier,
            &keys,
        ));
        let wave = valid(KernelSafetyCause::same_time_wave_exhausted(
            nonzero(1),
            2,
            frontier,
            &keys,
        ));
        let terminal_moment = moment(u64::MAX, u64::MAX);
        let terminal = valid(KernelSafetyCause::terminal_clock_exhausted(
            terminal_moment,
            &due_keys(terminal_moment, 1),
        ));

        assert_eq!(due.canonical_tag(), 0);
        assert_eq!(evaluable.canonical_tag(), 1);
        assert_eq!(wave.canonical_tag(), 2);
        assert_eq!(terminal.canonical_tag(), 3);
    }
}
