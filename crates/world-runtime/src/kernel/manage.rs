use core::fmt;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, SimMoment};
use world_model::{ActionEvaluationInvocationId, CommandId, CommandSource};

use crate::action_evaluation::ActionEvaluationFallbackCause;
use crate::authority::AuthorityRecordId;
use crate::kernel::InputId;
use crate::session::SessionMode;

/// Canonical schema of a session-management request.
pub const MANAGEMENT_REQUEST_SCHEMA_VERSION: u16 = 2;

const MANAGEMENT_REQUEST_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("management-request-v2") {
        Ok(domain) => domain,
        Err(_) => panic!("management request domain must be valid"),
    };

/// Host-issued identity of one session-management request.
///
/// Zero is a valid value. Issuance policy belongs to the management client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManagementRequestId(u64);

impl ManagementRequestId {
    /// Constructs an identity from its exact client-local value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact client-local value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One typed retained-request frontier selected for authoritative retirement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LedgerRetirement {
    /// Retire the input-request namespace through the target identity.
    InputThrough(InputId),
    /// Retire the host-management namespace through the target identity.
    ManagementThrough(ManagementRequestId),
    /// Retire one command source through its source-local target identity.
    CommandThrough {
        /// Command namespace whose frontier advances.
        source: CommandSource,
        /// Last command identity included in the retired prefix.
        command: CommandId,
    },
}

impl LedgerRetirement {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::InputThrough(_) => 0,
            Self::ManagementThrough(_) => 1,
            Self::CommandThrough { .. } => 2,
        }
    }
}

/// Host-owned disposition of one pending deferred action evaluation.
///
/// Engine-owned causes such as invalid results and artifact rejection are not
/// representable through this management boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionEvaluationManagementDisposition {
    /// Stop waiting because the host explicitly cancelled evaluation.
    Cancel,
    /// Stop waiting because the host recorded an external timeout.
    Timeout,
    /// Stop waiting because the evaluator or its host failed.
    HostFailure,
}

impl ActionEvaluationManagementDisposition {
    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::Cancel => 0,
            Self::Timeout => 1,
            Self::HostFailure => 2,
        }
    }

    pub(crate) const fn fallback_cause(self) -> ActionEvaluationFallbackCause {
        match self {
            Self::Cancel => ActionEvaluationFallbackCause::Cancelled,
            Self::Timeout => ActionEvaluationFallbackCause::TimedOut,
            Self::HostFailure => ActionEvaluationFallbackCause::HostFailure,
        }
    }
}

/// Closed host-management operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SessionManagement {
    /// Suspend delivery while retaining scheduled work.
    Pause,
    /// Continue delivery from a paused session.
    Resume,
    /// Place the session in host-directed quarantine while retaining scheduled work.
    Quarantine,
    /// Mark the session as host-directed failed while retaining scheduled work.
    Fail,
    /// Advance exactly one retained-request retirement frontier.
    Retire(LedgerRetirement),
    /// Prevent future ingress from scheduling commands before an exact frontier.
    SealAdmissionThrough(SimMoment),
    /// Release one pending evaluator obligation into its fixed later fallback.
    ResolveActionEvaluation {
        /// Exact logical invocation being resolved.
        invocation: ActionEvaluationInvocationId,
        /// Host-owned reason for ending external evaluation.
        disposition: ActionEvaluationManagementDisposition,
    },
}

impl SessionManagement {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Pause => 0,
            Self::Resume => 1,
            Self::Retire(_) => 2,
            Self::SealAdmissionThrough(_) => 3,
            Self::Quarantine => 4,
            Self::Fail => 5,
            Self::ResolveActionEvaluation { .. } => 6,
        }
    }

    pub(crate) const fn resulting_mode(self) -> Option<SessionMode> {
        match self {
            Self::Pause => Some(SessionMode::Paused),
            Self::Resume => Some(SessionMode::Running),
            Self::Quarantine => Some(SessionMode::Quarantined),
            Self::Fail => Some(SessionMode::Failed),
            Self::Retire(_)
            | Self::SealAdmissionThrough(_)
            | Self::ResolveActionEvaluation { .. } => None,
        }
    }
}

/// Canonical identity of one session-management request body.
///
/// [`ManagementRequestId`] is deliberately omitted so identity reuse can be
/// classified independently from exact retries.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManagementRequestFingerprint(ContentDigest);

impl ManagementRequestFingerprint {
    fn derive(operation: SessionManagement) -> Self {
        Self(ContentDigest::of_canonical(&management_request_bytes(
            operation,
        )))
    }

    /// Returns the exact fingerprint bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Consumes the fingerprint and returns its exact bytes.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 32] {
        self.0.into_bytes()
    }
}

impl fmt::Display for ManagementRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl fmt::Debug for ManagementRequestFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ManagementRequestFingerprint({self})")
    }
}

/// One singular request to change session control state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManageRequest {
    id: ManagementRequestId,
    operation: SessionManagement,
    fingerprint: ManagementRequestFingerprint,
}

impl ManageRequest {
    /// Captures one closed management operation and derives its fingerprint.
    #[must_use]
    pub fn new(id: ManagementRequestId, operation: SessionManagement) -> Self {
        Self {
            id,
            operation,
            fingerprint: ManagementRequestFingerprint::derive(operation),
        }
    }

    /// Returns the request identity.
    #[must_use]
    pub const fn id(self) -> ManagementRequestId {
        self.id
    }

    /// Returns the requested operation.
    #[must_use]
    pub const fn operation(self) -> SessionManagement {
        self.operation
    }

    /// Returns the canonical request fingerprint.
    #[must_use]
    pub const fn fingerprint(self) -> ManagementRequestFingerprint {
        self.fingerprint
    }
}

/// Retained successful result of one session-management request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManageOutcome {
    /// The session entered the paused mode.
    #[non_exhaustive]
    Paused {
        /// Record that captured the mode change and ledger result.
        record: AuthorityRecordId,
    },
    /// The session entered the running mode.
    #[non_exhaustive]
    Resumed {
        /// Record that captured the mode change and ledger result.
        record: AuthorityRecordId,
    },
    /// The session entered the quarantined mode.
    #[non_exhaustive]
    Quarantined {
        /// Record that captured the mode change and ledger result.
        record: AuthorityRecordId,
    },
    /// The session entered the failed mode.
    #[non_exhaustive]
    Failed {
        /// Record that captured the mode change and ledger result.
        record: AuthorityRecordId,
    },
    /// One retained-request frontier advanced without changing session mode.
    #[non_exhaustive]
    Retired {
        /// Record that captured the frontier change and ledger result.
        record: AuthorityRecordId,
        /// Exact typed frontier change installed by the record.
        retirement: LedgerRetirement,
    },
    /// The session's ingress admission frontier advanced.
    #[non_exhaustive]
    AdmissionSealed {
        /// Record that captured the frontier change and ledger result.
        record: AuthorityRecordId,
        /// First exact moment at which new command ingress may be scheduled.
        frontier: SimMoment,
    },
    /// One pending action evaluation entered its fixed later fallback.
    #[non_exhaustive]
    ActionEvaluationFallbackScheduled {
        /// Record that captured management and scheduled fallback.
        record: AuthorityRecordId,
        /// Exact logical invocation being resolved.
        invocation: ActionEvaluationInvocationId,
        /// Host-owned reason retained by management.
        disposition: ActionEvaluationManagementDisposition,
    },
}

impl ManageOutcome {
    #[must_use]
    pub(crate) const fn applied(record: AuthorityRecordId, operation: SessionManagement) -> Self {
        match operation {
            SessionManagement::Pause => Self::Paused { record },
            SessionManagement::Resume => Self::Resumed { record },
            SessionManagement::Quarantine => Self::Quarantined { record },
            SessionManagement::Fail => Self::Failed { record },
            SessionManagement::Retire(retirement) => Self::Retired { record, retirement },
            SessionManagement::SealAdmissionThrough(frontier) => {
                Self::AdmissionSealed { record, frontier }
            }
            SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition,
            } => Self::ActionEvaluationFallbackScheduled {
                record,
                invocation,
                disposition,
            },
        }
    }

    /// Returns the record that captured the management result.
    #[must_use]
    pub const fn record(self) -> AuthorityRecordId {
        match self {
            Self::Paused { record }
            | Self::Resumed { record }
            | Self::Quarantined { record }
            | Self::Failed { record }
            | Self::Retired { record, .. }
            | Self::AdmissionSealed { record, .. }
            | Self::ActionEvaluationFallbackScheduled { record, .. } => record,
        }
    }

    /// Returns the mode installed by this outcome, if it changed session mode.
    #[must_use]
    pub const fn resulting_mode(self) -> Option<SessionMode> {
        match self {
            Self::Paused { .. } => Some(SessionMode::Paused),
            Self::Resumed { .. } => Some(SessionMode::Running),
            Self::Quarantined { .. } => Some(SessionMode::Quarantined),
            Self::Failed { .. } => Some(SessionMode::Failed),
            Self::Retired { .. }
            | Self::AdmissionSealed { .. }
            | Self::ActionEvaluationFallbackScheduled { .. } => None,
        }
    }

    /// Returns the installed retirement delta, if this outcome advanced a frontier.
    #[must_use]
    pub const fn retirement(self) -> Option<LedgerRetirement> {
        match self {
            Self::Retired { retirement, .. } => Some(retirement),
            Self::Paused { .. }
            | Self::Resumed { .. }
            | Self::Quarantined { .. }
            | Self::Failed { .. }
            | Self::AdmissionSealed { .. }
            | Self::ActionEvaluationFallbackScheduled { .. } => None,
        }
    }

    /// Returns the installed ingress-admission frontier, if this outcome sealed admission.
    #[must_use]
    pub const fn admission_frontier(self) -> Option<SimMoment> {
        match self {
            Self::AdmissionSealed { frontier, .. } => Some(frontier),
            Self::Paused { .. }
            | Self::Resumed { .. }
            | Self::Quarantined { .. }
            | Self::Failed { .. }
            | Self::Retired { .. }
            | Self::ActionEvaluationFallbackScheduled { .. } => None,
        }
    }

    /// Returns an action-evaluation management result, if this outcome has one.
    #[must_use]
    pub const fn action_evaluation_resolution(
        self,
    ) -> Option<(
        ActionEvaluationInvocationId,
        ActionEvaluationManagementDisposition,
    )> {
        match self {
            Self::ActionEvaluationFallbackScheduled {
                invocation,
                disposition,
                ..
            } => Some((invocation, disposition)),
            Self::Paused { .. }
            | Self::Resumed { .. }
            | Self::Quarantined { .. }
            | Self::Failed { .. }
            | Self::Retired { .. }
            | Self::AdmissionSealed { .. } => None,
        }
    }
}

fn management_request_bytes(operation: SessionManagement) -> CanonicalBytes {
    let mut writer = CanonicalWriter::new(MANAGEMENT_REQUEST_DOMAIN);
    writer.write_u16(MANAGEMENT_REQUEST_SCHEMA_VERSION);
    writer.write_discriminant(operation.canonical_tag());
    if let SessionManagement::Retire(retirement) = operation {
        writer.write_discriminant(retirement.canonical_tag());
        match retirement {
            LedgerRetirement::InputThrough(target) => writer.write_u64(target.get()),
            LedgerRetirement::ManagementThrough(target) => writer.write_u64(target.get()),
            LedgerRetirement::CommandThrough { source, command } => {
                if writer.write_bytes(source.as_bytes()).is_err() {
                    unreachable!("fixed-width command source must fit canonical encoding");
                }
                writer.write_u64(command.get());
            }
        }
    } else if let SessionManagement::SealAdmissionThrough(frontier) = operation {
        writer.write_u64(frontier.time().ticks());
        writer.write_u64(frontier.microstep().get());
    } else if let SessionManagement::ResolveActionEvaluation {
        invocation,
        disposition,
    } = operation
    {
        if writer.write_bytes(invocation.as_bytes()).is_err() {
            unreachable!("fixed-width action-evaluation invocation must fit canonical encoding");
        }
        writer.write_discriminant(disposition.canonical_tag());
    }
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Microstep, SimTime};

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
    fn management_preimages_are_byte_complete() {
        let paused = management_request_bytes(SessionManagement::Pause);
        let resumed = management_request_bytes(SessionManagement::Resume);
        let quarantined = management_request_bytes(SessionManagement::Quarantine);
        let failed = management_request_bytes(SessionManagement::Fail);
        let admission = management_request_bytes(SessionManagement::SealAdmissionThrough(
            SimMoment::new(SimTime::from_ticks(4), Microstep::new(2)),
        ));
        let action_evaluation =
            management_request_bytes(SessionManagement::ResolveActionEvaluation {
                invocation: ActionEvaluationInvocationId::from_bytes([0x51; 32]),
                disposition: ActionEvaluationManagementDisposition::Timeout,
            });

        assert_eq!(
            hex(paused.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000156d616e6167656d656e742d726571756573742d7632000200000000"
        );
        assert_eq!(
            hex(resumed.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000156d616e6167656d656e742d726571756573742d7632000200000001"
        );
        assert_eq!(
            hex(quarantined.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000156d616e6167656d656e742d726571756573742d7632000200000004"
        );
        assert_eq!(
            hex(failed.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000156d616e6167656d656e742d726571756573742d7632000200000005"
        );
        assert_eq!(
            hex(admission.as_bytes()),
            "776f726c642d63616e6f6e6963616c2d763100000000000000156d616e6167656d656e742d726571756573742d763200020000000300000000000000040000000000000002"
        );
        assert_eq!(
            hex(action_evaluation.as_bytes()),
            concat!(
                "776f726c642d63616e6f6e6963616c2d763100000000000000156d616e6167656d656e742d726571756573742d763200",
                "020000000600000000000000205151515151515151515151515151515151515151515151515151515151515151000000",
                "01",
            )
        );
    }

    #[test]
    fn management_identity_is_omitted_but_operation_is_committed() {
        let pause = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Pause);
        let retry = ManageRequest::new(ManagementRequestId::new(9), SessionManagement::Pause);
        let resume = ManageRequest::new(ManagementRequestId::new(0), SessionManagement::Resume);

        assert_eq!(ManagementRequestId::new(0).get(), 0);
        assert_eq!(pause.fingerprint(), retry.fingerprint());
        assert_ne!(pause.fingerprint(), resume.fingerprint());
    }

    #[test]
    fn outcomes_encode_the_resulting_mode_in_their_variant() {
        let record = AuthorityRecordId::from_bytes([0x31; 32]);
        let paused = ManageOutcome::applied(record, SessionManagement::Pause);
        let resumed = ManageOutcome::applied(record, SessionManagement::Resume);
        let quarantined = ManageOutcome::applied(record, SessionManagement::Quarantine);
        let failed = ManageOutcome::applied(record, SessionManagement::Fail);
        let retirement = LedgerRetirement::InputThrough(InputId::new(7));
        let retired = ManageOutcome::applied(record, SessionManagement::Retire(retirement));
        let frontier = SimMoment::new(SimTime::from_ticks(9), Microstep::new(3));
        let sealed =
            ManageOutcome::applied(record, SessionManagement::SealAdmissionThrough(frontier));
        let invocation = ActionEvaluationInvocationId::from_bytes([0x41; 32]);
        let disposition = ActionEvaluationManagementDisposition::Timeout;
        let action = ManageOutcome::applied(
            record,
            SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition,
            },
        );

        assert_eq!(paused.record(), record);
        assert_eq!(paused.resulting_mode(), Some(SessionMode::Paused));
        assert_eq!(paused.retirement(), None);
        assert_eq!(paused.admission_frontier(), None);
        assert_eq!(resumed.record(), record);
        assert_eq!(resumed.resulting_mode(), Some(SessionMode::Running));
        assert_eq!(resumed.retirement(), None);
        assert_eq!(resumed.admission_frontier(), None);
        assert_eq!(quarantined.resulting_mode(), Some(SessionMode::Quarantined));
        assert_eq!(quarantined.retirement(), None);
        assert_eq!(quarantined.admission_frontier(), None);
        assert_eq!(failed.resulting_mode(), Some(SessionMode::Failed));
        assert_eq!(failed.retirement(), None);
        assert_eq!(failed.admission_frontier(), None);
        assert_eq!(retired.record(), record);
        assert_eq!(retired.resulting_mode(), None);
        assert_eq!(retired.retirement(), Some(retirement));
        assert_eq!(retired.admission_frontier(), None);
        assert_eq!(sealed.record(), record);
        assert_eq!(sealed.resulting_mode(), None);
        assert_eq!(sealed.retirement(), None);
        assert_eq!(sealed.admission_frontier(), Some(frontier));
        assert_eq!(action.record(), record);
        assert_eq!(action.resulting_mode(), None);
        assert_eq!(action.retirement(), None);
        assert_eq!(action.admission_frontier(), None);
        assert_eq!(
            action.action_evaluation_resolution(),
            Some((invocation, disposition))
        );
        assert_eq!(
            SessionManagement::Pause.resulting_mode(),
            Some(SessionMode::Paused)
        );
        assert_eq!(
            SessionManagement::Resume.resulting_mode(),
            Some(SessionMode::Running)
        );
        assert_eq!(
            SessionManagement::Quarantine.resulting_mode(),
            Some(SessionMode::Quarantined)
        );
        assert_eq!(
            SessionManagement::Fail.resulting_mode(),
            Some(SessionMode::Failed)
        );
        assert_eq!(SessionManagement::Retire(retirement).resulting_mode(), None);
        assert_eq!(
            SessionManagement::SealAdmissionThrough(frontier).resulting_mode(),
            None
        );
        assert_eq!(
            SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition,
            }
            .resulting_mode(),
            None
        );
    }

    #[test]
    fn retirement_fingerprints_commit_the_typed_frontier() {
        let input = ManageRequest::new(
            ManagementRequestId::new(10),
            SessionManagement::Retire(LedgerRetirement::InputThrough(InputId::new(4))),
        );
        let management = ManageRequest::new(
            ManagementRequestId::new(10),
            SessionManagement::Retire(LedgerRetirement::ManagementThrough(
                ManagementRequestId::new(4),
            )),
        );
        let command = ManageRequest::new(
            ManagementRequestId::new(10),
            SessionManagement::Retire(LedgerRetirement::CommandThrough {
                source: CommandSource::from_bytes([0x41; 32]),
                command: CommandId::new(4),
            }),
        );
        let other_command = ManageRequest::new(
            ManagementRequestId::new(99),
            SessionManagement::Retire(LedgerRetirement::CommandThrough {
                source: CommandSource::from_bytes([0x42; 32]),
                command: CommandId::new(4),
            }),
        );
        let admission = ManageRequest::new(
            ManagementRequestId::new(10),
            SessionManagement::SealAdmissionThrough(SimMoment::new(
                SimTime::from_ticks(4),
                Microstep::new(2),
            )),
        );
        assert_ne!(input.fingerprint(), management.fingerprint());
        assert_ne!(management.fingerprint(), command.fingerprint());
        assert_ne!(command.fingerprint(), other_command.fingerprint());
        assert_ne!(other_command.fingerprint(), admission.fingerprint());
    }

    #[test]
    fn terminal_management_fingerprints_are_distinct() {
        let quarantine =
            ManageRequest::new(ManagementRequestId::new(10), SessionManagement::Quarantine);
        let failure = ManageRequest::new(ManagementRequestId::new(10), SessionManagement::Fail);

        assert_ne!(quarantine.fingerprint(), failure.fingerprint());
        assert_ne!(
            quarantine.fingerprint(),
            ManageRequest::new(ManagementRequestId::new(10), SessionManagement::Pause)
                .fingerprint()
        );
    }

    #[test]
    fn action_evaluation_management_commits_invocation_and_closed_disposition() {
        let id = ManagementRequestId::new(11);
        let invocation = ActionEvaluationInvocationId::from_bytes([0x51; 32]);
        let other_invocation = ActionEvaluationInvocationId::from_bytes([0x52; 32]);
        let cancelled = ManageRequest::new(
            id,
            SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition: ActionEvaluationManagementDisposition::Cancel,
            },
        );
        let timed_out = ManageRequest::new(
            id,
            SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition: ActionEvaluationManagementDisposition::Timeout,
            },
        );
        let failed = ManageRequest::new(
            id,
            SessionManagement::ResolveActionEvaluation {
                invocation,
                disposition: ActionEvaluationManagementDisposition::HostFailure,
            },
        );
        let other = ManageRequest::new(
            id,
            SessionManagement::ResolveActionEvaluation {
                invocation: other_invocation,
                disposition: ActionEvaluationManagementDisposition::Cancel,
            },
        );

        assert_ne!(cancelled.fingerprint(), timed_out.fingerprint());
        assert_ne!(timed_out.fingerprint(), failed.fingerprint());
        assert_ne!(cancelled.fingerprint(), other.fingerprint());
        assert_eq!(
            [
                ActionEvaluationManagementDisposition::Cancel,
                ActionEvaluationManagementDisposition::Timeout,
                ActionEvaluationManagementDisposition::HostFailure,
            ]
            .map(ActionEvaluationManagementDisposition::fallback_cause),
            [
                ActionEvaluationFallbackCause::Cancelled,
                ActionEvaluationFallbackCause::TimedOut,
                ActionEvaluationFallbackCause::HostFailure,
            ]
        );
    }
}
