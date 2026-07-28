use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, SimMoment};

use crate::authority::{AuthorityCursor, CumulativeAuthorityHash};
use crate::execution::{
    ExecutionSpecId, FinalizationPolicyV1, SemanticTerminationReasonV1, TerminationClauseId,
    TerminationContractV1,
};

use super::{AttemptBinding, AttemptDisposition, AttemptDispositionId, RunAttemptId};

const TRAJECTORY_SCHEMA_VERSION: u16 = 1;
#[cfg(test)]
const RUN_FINALIZATION_SCHEMA_VERSION: u16 = 1;

const TRAJECTORY_DOMAIN: CanonicalDomain = match CanonicalDomain::new("trajectory-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("trajectory domain must be valid"),
};

#[cfg(test)]
const RUN_FINALIZATION_DOMAIN: CanonicalDomain = match CanonicalDomain::new("run-finalization-v1") {
    Ok(domain) => domain,
    Err(_) => panic!("run finalization domain must be valid"),
};

/// Semantic identity of one authoritative trajectory prefix.
///
/// Physical attempt identity is excluded, so independently controlled
/// reproductions of the same execution and history have one trajectory ID.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrajectoryId([u8; 32]);

impl TrajectoryId {
    /// Constructs a fixed-width identity decoded by a result owner.
    ///
    /// Runtime recomputes this value before accepting decoded finalization
    /// evidence.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn derive(execution: ExecutionSpecId, cumulative: CumulativeAuthorityHash) -> Self {
        Self(ContentDigest::of_canonical(&trajectory_bytes(execution, cumulative)).into_bytes())
    }

    /// Returns the exact identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Consumes the identity and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for TrajectoryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for TrajectoryId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TrajectoryId({self})")
    }
}

/// Correlated terminal reason and its exact retained evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunFinalizationCause {
    /// The configured simulation-moment clause selected the prefix.
    ReachedConfiguredMoment {
        /// Exact clause that selected finalization.
        clause: TerminationClauseId,
    },
    /// The host cancelled the physical attempt.
    Cancelled {
        /// Retained cancellation disposition.
        disposition: AttemptDispositionId,
    },
    /// The host-side evaluation budget was exhausted.
    HostBudgetExceeded {
        /// Retained failure disposition.
        disposition: AttemptDispositionId,
    },
    /// A declared external evaluator failed.
    ExternalFailure {
        /// Retained failure disposition.
        disposition: AttemptDispositionId,
    },
    /// Engine coordination failed.
    EngineFailure {
        /// Retained failure disposition.
        disposition: AttemptDispositionId,
    },
}

impl RunFinalizationCause {
    pub(crate) const fn semantic(
        clause: TerminationClauseId,
        reason: SemanticTerminationReasonV1,
    ) -> Self {
        match reason {
            SemanticTerminationReasonV1::ReachedConfiguredMoment => {
                Self::ReachedConfiguredMoment { clause }
            }
        }
    }

    pub(crate) fn from_disposition(disposition: AttemptDisposition) -> Self {
        let id = disposition.id();
        match disposition {
            AttemptDisposition::CancelRequested { .. } => Self::Cancelled { disposition: id },
            AttemptDisposition::HostBudgetExceeded => Self::HostBudgetExceeded { disposition: id },
            AttemptDisposition::ExternalFailure => Self::ExternalFailure { disposition: id },
            AttemptDisposition::EngineFailure => Self::EngineFailure { disposition: id },
        }
    }

    #[cfg(test)]
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::ReachedConfiguredMoment { .. } => 0,
            Self::Cancelled { .. } => 1,
            Self::HostBudgetExceeded { .. } => 2,
            Self::ExternalFailure { .. } => 3,
            Self::EngineFailure { .. } => 4,
        }
    }
}

/// Immutable terminal selection for one physical run attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunFinalization {
    attempt: RunAttemptId,
    terminal: AuthorityCursor,
    cause: RunFinalizationCause,
    trajectory: TrajectoryId,
}

impl RunFinalization {
    pub(crate) fn new(
        binding: AttemptBinding,
        terminal: AuthorityCursor,
        cause: RunFinalizationCause,
    ) -> Result<Self, FinalizationBindingError> {
        if terminal.epoch().execution() != binding.execution()
            || terminal.epoch().lineage() != binding.lineage()
        {
            return Err(FinalizationBindingError);
        }
        Ok(Self {
            attempt: binding.attempt(),
            terminal,
            cause,
            trajectory: TrajectoryId::derive(binding.execution(), terminal.cumulative()),
        })
    }

    /// Returns the physical attempt that selected this prefix.
    #[must_use]
    pub const fn attempt(self) -> RunAttemptId {
        self.attempt
    }

    /// Returns the exact terminal authority cursor.
    #[must_use]
    pub const fn terminal(self) -> AuthorityCursor {
        self.terminal
    }

    /// Returns the correlated terminal cause and evidence.
    #[must_use]
    pub const fn cause(self) -> RunFinalizationCause {
        self.cause
    }

    /// Returns the semantic identity of the selected trajectory.
    #[must_use]
    pub const fn trajectory(self) -> TrajectoryId {
        self.trajectory
    }

    #[cfg(test)]
    pub(crate) fn canonical_bytes(self) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(RUN_FINALIZATION_DOMAIN);
        writer.write_u16(RUN_FINALIZATION_SCHEMA_VERSION);
        write_fixed_bytes(&mut writer, self.attempt.as_bytes());
        write_owned_bytes(&mut writer, self.terminal.canonical_bytes().as_bytes());
        write_cause(&mut writer, self.cause);
        write_fixed_bytes(&mut writer, self.trajectory.as_bytes());
        writer.finish()
    }
}

/// Projects the unique terminal selection chosen by immutable execution policy.
///
/// Attempt phase storage calls this pure function; it does not decide
/// disposition precedence or interpret semantic termination itself.
pub(crate) fn project_run_finalization(
    binding: AttemptBinding,
    terminal: AuthorityCursor,
    now: SimMoment,
    contract: TerminationContractV1,
    policy: FinalizationPolicyV1,
    disposition: Option<AttemptDisposition>,
) -> Result<Option<RunFinalization>, FinalizationBindingError> {
    let cause = match policy {
        FinalizationPolicyV1::DispositionFirst => match disposition {
            Some(disposition) => RunFinalizationCause::from_disposition(disposition),
            None => {
                let Some(reason) = contract.evaluate(now) else {
                    return Ok(None);
                };
                let Some(clause) = contract.clause_id() else {
                    unreachable!("a selected semantic reason must have a clause identity");
                };
                RunFinalizationCause::semantic(clause, reason)
            }
        },
    };
    RunFinalization::new(binding, terminal, cause).map(Some)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FinalizationBindingError;

fn trajectory_bytes(
    execution: ExecutionSpecId,
    cumulative: CumulativeAuthorityHash,
) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(TRAJECTORY_DOMAIN);
    writer.write_u16(TRAJECTORY_SCHEMA_VERSION);
    write_fixed_bytes(&mut writer, execution.as_bytes());
    write_fixed_bytes(&mut writer, cumulative.as_bytes());
    writer.finish()
}

#[cfg(test)]
fn write_cause(writer: &mut CanonicalWriter, cause: RunFinalizationCause) {
    writer.write_discriminant(cause.canonical_tag());
    match cause {
        RunFinalizationCause::ReachedConfiguredMoment { clause } => {
            write_fixed_bytes(writer, clause.as_bytes());
            writer.write_discriminant(0);
        }
        RunFinalizationCause::Cancelled { disposition }
        | RunFinalizationCause::HostBudgetExceeded { disposition }
        | RunFinalizationCause::ExternalFailure { disposition }
        | RunFinalizationCause::EngineFailure { disposition } => {
            write_fixed_bytes(writer, disposition.as_bytes());
        }
    }
}

fn write_fixed_bytes(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width identity length must fit the canonical protocol");
    }
}

#[cfg(test)]
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
    use core::fmt::Debug;

    use world_core::SimMoment;
    use world_model::{AcceptedState, AgencyState, DomainState, EpistemicState, SocialState};

    use crate::control::test_support;
    use crate::execution::{
        CanonicalExecutionSpecV1, ExecutionConfigArtifactV3, ExecutionSemanticsManifestV1,
        ExternalInputBindingV1, InitialStateRootV1, ResolvedExecutionClosureManifestV1, RootSeed,
        TerminationContractV1,
    };
    use crate::session::SessionMode;

    use super::super::{AttemptAuthorityDomainId, AttemptCreation, AttemptDisposition, AttemptKey};
    use super::*;

    fn valid<T, E: Debug>(result: Result<T, E>) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("finalization fixture must be valid: {error:?}"),
        }
    }

    fn closure(seed: u8) -> ResolvedExecutionClosureManifestV1 {
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
            RootSeed::from_bytes([seed; 32]),
            TerminationContractV1::Never,
            ExternalInputBindingV1::HostSerialized,
        );
        valid(ResolvedExecutionClosureManifestV1::bind(
            root,
            specification,
            semantics,
        ))
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

    #[test]
    fn finalization_and_trajectory_have_frozen_vectors() {
        let closure = closure(0x61);
        let creation = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([0x11; 32]),
            AttemptKey::from_bytes([0x21; 32]),
            &closure,
        );
        let finalization = valid(RunFinalization::new(
            creation.binding(),
            closure.root_cursor(),
            RunFinalizationCause::from_disposition(AttemptDisposition::EngineFailure),
        ));

        assert_eq!(
            finalization.trajectory().to_string(),
            "e5de2b88ec6bc767e448e3d216e610308f1854322337788fe7a3e976b0dff95f"
        );
        assert_eq!(
            hex(finalization.canonical_bytes().as_bytes()),
            "776f726c642d63616e6f6e6963616c2d7631000000000000001372756e2d66696e616c697a6174696f6e2d7631000100000000000000205b04ee472d1e969012759502c04d118accbc9d4b0579e089900bc8c610ca667600000000000000d3776f726c642d63616e6f6e6963616c2d76310000000000000013617574686f726974792d637572736f722d763100010000000000000020b11dbda2fa2028edbcd0b4d91d3f44e515c5108300d1b890d19b4dbdb91bb48700000000000000202a68d1d8125de8bf0943b07745f316eb6ee7a5a91182b7c62530a45e311607a6000000000000000000000020a69ccc2caf60d5c3e776408d80491d5f885006191d59ff4a31dbaaccfc9a102600000000000000204d8475f8965ba58445250fe767dc55c4f8210550deaa99a4c9bfe788af1bca3800000004000000000000002097e95240bfe80c68061c02fb7c540308c46a5f8cfdab6e16a882ded2db0b97790000000000000020e5de2b88ec6bc767e448e3d216e610308f1854322337788fe7a3e976b0dff95f"
        );
    }

    #[test]
    fn trajectory_excludes_physical_attempt_identity() {
        let closure = closure(0x62);
        let first = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([0x31; 32]),
            AttemptKey::from_bytes([0x41; 32]),
            &closure,
        );
        let second = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([0x32; 32]),
            AttemptKey::from_bytes([0x42; 32]),
            &closure,
        );
        let cause = RunFinalizationCause::from_disposition(AttemptDisposition::EngineFailure);
        let first_finalization = valid(RunFinalization::new(
            first.binding(),
            closure.root_cursor(),
            cause,
        ));
        let second_finalization = valid(RunFinalization::new(
            second.binding(),
            closure.root_cursor(),
            cause,
        ));

        assert_ne!(first_finalization.attempt(), second_finalization.attempt());
        assert_eq!(
            first_finalization.trajectory(),
            second_finalization.trajectory()
        );
    }

    #[test]
    fn finalization_rejects_a_cursor_from_another_execution() {
        let bound = closure(0x63);
        let other = closure(0x64);
        let creation = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([0x33; 32]),
            AttemptKey::from_bytes([0x43; 32]),
            &bound,
        );

        assert_eq!(
            RunFinalization::new(
                creation.binding(),
                other.root_cursor(),
                RunFinalizationCause::from_disposition(AttemptDisposition::ExternalFailure),
            ),
            Err(FinalizationBindingError)
        );
    }

    #[test]
    fn projection_applies_the_manifest_selected_disposition_precedence() {
        let closure = closure(0x65);
        let creation = AttemptCreation::derive(
            AttemptAuthorityDomainId::from_bytes([0x35; 32]),
            AttemptKey::from_bytes([0x45; 32]),
            &closure,
        );
        let contract = TerminationContractV1::at_or_after_moment(SimMoment::ORIGIN);

        let semantic = valid(project_run_finalization(
            creation.binding(),
            closure.root_cursor(),
            SimMoment::ORIGIN,
            contract,
            FinalizationPolicyV1::DispositionFirst,
            None,
        ))
        .unwrap_or_else(|| panic!("the root-time semantic clause must select finalization"));
        assert!(matches!(
            semantic.cause(),
            RunFinalizationCause::ReachedConfiguredMoment { .. }
        ));

        let disposition = valid(project_run_finalization(
            creation.binding(),
            closure.root_cursor(),
            SimMoment::ORIGIN,
            contract,
            FinalizationPolicyV1::DispositionFirst,
            Some(AttemptDisposition::EngineFailure),
        ))
        .unwrap_or_else(|| panic!("the retained disposition must select finalization"));
        assert!(matches!(
            disposition.cause(),
            RunFinalizationCause::EngineFailure { .. }
        ));
    }
}
