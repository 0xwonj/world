use core::fmt;
use std::collections::BTreeMap;

use world_core::{CanonicalBytes, CanonicalDomain, CanonicalWriter, ContentDigest, SimMoment};
use world_model::{
    ActionEvaluationGeneration, ActionEvaluationInvocationId, ActionOpportunity,
    ActionOpportunityId, ActionOpportunityState, ActionOpportunityVersion,
};

use crate::authority::{AuthorityCursor, AuthorityRecordId};
use crate::execution::{
    DeferredActionAdmissionModeV1, DeferredActionControlV1, DeferredActionFallbackV1,
    LifecycleImplementationId,
};
use crate::scheduler::{SchedulerKey, SchedulerLaneV2};

/// Canonical schema of one retained action-evaluation artifact envelope.
const ACTION_EVALUATION_ARTIFACT_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of one action-evaluation result-capture request.
const ACTION_EVALUATION_CAPTURE_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of the action-evaluation capture replay ledger.
#[cfg(test)]
const ACTION_EVALUATION_CAPTURE_LEDGER_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of one retained action-evaluation invocation.
const ACTION_EVALUATION_INVOCATION_SCHEMA_VERSION: u16 = 1;

/// Canonical schema of the action-evaluation invocation ledger.
#[cfg(test)]
const ACTION_EVALUATION_LEDGER_SCHEMA_VERSION: u16 = 1;

const ACTION_EVALUATION_ARTIFACT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-artifact-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation artifact domain must be valid"),
    };
const ACTION_EVALUATION_REQUEST_ID_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-request-id-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation request identity domain must be valid"),
    };
const ACTION_EVALUATION_RESULT_ID_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-result-id-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation result identity domain must be valid"),
    };
const ACTION_EVALUATION_CAPTURE_FINGERPRINT_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-capture-fingerprint-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation capture fingerprint domain must be valid"),
    };
#[cfg(test)]
const ACTION_EVALUATION_CAPTURE_LEDGER_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-capture-ledger-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation capture ledger domain must be valid"),
    };
const ACTION_EVALUATION_INVOCATION_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-invocation-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation invocation domain must be valid"),
    };
#[cfg(test)]
const ACTION_EVALUATION_LEDGER_DOMAIN: CanonicalDomain =
    match CanonicalDomain::new("action-evaluation-ledger-v1") {
        Ok(domain) => domain,
        Err(_) => panic!("action evaluation ledger domain must be valid"),
    };

macro_rules! fixed_identity {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Constructs a fixed-width value decoded by its owning protocol.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
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

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(formatter, "{byte:02x}")?;
                }
                Ok(())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({self})", stringify!($name))
            }
        }
    };
}

fixed_identity!(
    /// Fixed schema identity of one action-evaluation artifact codec.
    ActionEvaluationArtifactSchemaId
);
fixed_identity!(
    /// Canonical digest of one role-bound action-evaluation artifact.
    ActionEvaluationArtifactDigest
);
fixed_identity!(
    /// Stable identity of one committed deferred-evaluation request.
    ActionEvaluationRequestId
);
fixed_identity!(
    /// Stable identity of one captured deferred-evaluation result.
    ActionEvaluationResultId
);
fixed_identity!(
    /// Canonical fingerprint of one result-capture request body.
    ActionEvaluationCaptureFingerprint
);
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActionEvaluationCaptureLedgerDigest([u8; 32]);
fixed_identity!(
    /// Canonical digest of one complete invocation record.
    ActionEvaluationInvocationDigest
);
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActionEvaluationInvocationLedgerDigest([u8; 32]);

/// Exact engine-facing binding for one captured result ready to interpret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionEvaluationResultReady {
    invocation: ActionEvaluationInvocationId,
    opportunity: ActionOpportunityId,
    expected_waiting_version: ActionOpportunityVersion,
    due: SimMoment,
}

impl ActionEvaluationResultReady {
    const fn new(
        invocation: ActionEvaluationInvocationId,
        opportunity: ActionOpportunityId,
        expected_waiting_version: ActionOpportunityVersion,
        due: SimMoment,
    ) -> Self {
        Self {
            invocation,
            opportunity,
            expected_waiting_version,
            due,
        }
    }

    /// Returns the captured logical invocation.
    #[must_use]
    pub const fn invocation(self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    /// Returns the waiting action opportunity.
    #[must_use]
    pub const fn opportunity(self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the exact waiting opportunity version.
    #[must_use]
    pub const fn expected_waiting_version(self) -> ActionOpportunityVersion {
        self.expected_waiting_version
    }

    /// Returns the exact result-interpretation moment.
    #[must_use]
    pub const fn due(self) -> SimMoment {
        self.due
    }
}

/// Closed later work owned by one retained action evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationWork {
    /// A captured result is ready for freshness and decision validation.
    ResultReady {
        /// Logical invocation being resumed.
        invocation: ActionEvaluationInvocationId,
        /// Opportunity that must still be waiting for this invocation.
        opportunity: ActionOpportunityId,
        /// Exact waiting opportunity version.
        expected_waiting_version: ActionOpportunityVersion,
        /// Exact later simulation moment.
        due: SimMoment,
    },
    /// A fixed failure fallback is ready to consume the waiting opportunity.
    Fallback {
        /// Logical invocation whose fallback is running.
        invocation: ActionEvaluationInvocationId,
        /// Opportunity that must still be waiting for this invocation.
        opportunity: ActionOpportunityId,
        /// Exact waiting opportunity version.
        expected_waiting_version: ActionOpportunityVersion,
        /// Concrete reason the fallback was scheduled.
        cause: ActionEvaluationFallbackCause,
        /// Exact later simulation moment.
        due: SimMoment,
    },
}

impl ActionEvaluationWork {
    pub(crate) const fn result_ready(
        invocation: ActionEvaluationInvocationId,
        opportunity: ActionOpportunityId,
        expected_waiting_version: ActionOpportunityVersion,
        due: SimMoment,
    ) -> Self {
        Self::ResultReady {
            invocation,
            opportunity,
            expected_waiting_version,
            due,
        }
    }

    pub(crate) const fn fallback(
        invocation: ActionEvaluationInvocationId,
        opportunity: ActionOpportunityId,
        expected_waiting_version: ActionOpportunityVersion,
        cause: ActionEvaluationFallbackCause,
        due: SimMoment,
    ) -> Self {
        Self::Fallback {
            invocation,
            opportunity,
            expected_waiting_version,
            cause,
            due,
        }
    }

    /// Returns the logical invocation.
    #[must_use]
    pub const fn invocation(self) -> ActionEvaluationInvocationId {
        match self {
            Self::ResultReady { invocation, .. } | Self::Fallback { invocation, .. } => invocation,
        }
    }

    /// Returns the waiting action opportunity.
    #[must_use]
    pub const fn opportunity(self) -> ActionOpportunityId {
        match self {
            Self::ResultReady { opportunity, .. } | Self::Fallback { opportunity, .. } => {
                opportunity
            }
        }
    }

    /// Returns the exact waiting opportunity version.
    #[must_use]
    pub const fn expected_waiting_version(self) -> ActionOpportunityVersion {
        match self {
            Self::ResultReady {
                expected_waiting_version,
                ..
            }
            | Self::Fallback {
                expected_waiting_version,
                ..
            } => expected_waiting_version,
        }
    }

    /// Returns the exact delivery moment.
    #[must_use]
    pub const fn due(self) -> SimMoment {
        match self {
            Self::ResultReady { due, .. } | Self::Fallback { due, .. } => due,
        }
    }

    /// Returns the fallback cause only for fallback work.
    #[must_use]
    pub const fn fallback_cause(self) -> Option<ActionEvaluationFallbackCause> {
        match self {
            Self::ResultReady { .. } => None,
            Self::Fallback { cause, .. } => Some(cause),
        }
    }

    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::ResultReady { .. } => 0,
            Self::Fallback { .. } => 1,
        }
    }

    pub(crate) const fn result_ready_binding(self) -> Option<ActionEvaluationResultReady> {
        match self {
            Self::ResultReady {
                invocation,
                opportunity,
                expected_waiting_version,
                due,
            } => Some(ActionEvaluationResultReady::new(
                invocation,
                opportunity,
                expected_waiting_version,
                due,
            )),
            Self::Fallback { .. } => None,
        }
    }
}

/// Host-issued identity in the action-evaluation result-capture namespace.
///
/// Zero is valid. Issuance policy belongs to the capture client, and this
/// namespace is deliberately distinct from ordinary command input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionEvaluationCaptureId(u64);

impl ActionEvaluationCaptureId {
    /// Constructs a capture identity from its namespace-local value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the exact namespace-local value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// How one captured evaluator result obtains simulation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationCaptureTiming {
    /// Runtime uses the invocation's retained blocking frontier.
    InvocationFrontier,
    /// The host names an explicit nonblocking simulation moment.
    HostScheduled(SimMoment),
}

/// Raw engine-to-runtime submission of one deferred evaluator result.
///
/// The runtime owns artifact bounds and canonical capture identity. Keeping
/// bytes raw at this boundary lets an oversized result become a recorded
/// fallback rather than an unrecorded construction error in the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationResultSubmission {
    capture: ActionEvaluationCaptureId,
    invocation: ActionEvaluationInvocationId,
    timing: ActionEvaluationCaptureTiming,
    result_schema: ActionEvaluationArtifactSchemaId,
    bytes: Box<[u8]>,
}

impl ActionEvaluationResultSubmission {
    /// Captures a result for an invocation whose retained frontier fixes time.
    #[must_use]
    pub fn at_invocation_frontier(
        capture: ActionEvaluationCaptureId,
        invocation: ActionEvaluationInvocationId,
        result_schema: ActionEvaluationArtifactSchemaId,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            capture,
            invocation,
            timing: ActionEvaluationCaptureTiming::InvocationFrontier,
            result_schema,
            bytes: bytes.into_boxed_slice(),
        }
    }

    /// Captures a result at one explicit nonblocking simulation moment.
    #[must_use]
    pub fn host_scheduled(
        capture: ActionEvaluationCaptureId,
        invocation: ActionEvaluationInvocationId,
        effective: SimMoment,
        result_schema: ActionEvaluationArtifactSchemaId,
        bytes: Vec<u8>,
    ) -> Self {
        Self {
            capture,
            invocation,
            timing: ActionEvaluationCaptureTiming::HostScheduled(effective),
            result_schema,
            bytes: bytes.into_boxed_slice(),
        }
    }

    /// Returns the capture namespace identity.
    #[must_use]
    pub const fn capture(&self) -> ActionEvaluationCaptureId {
        self.capture
    }

    /// Returns the retained invocation being answered.
    #[must_use]
    pub const fn invocation(&self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    /// Returns how simulation time is assigned.
    #[must_use]
    pub const fn timing(&self) -> ActionEvaluationCaptureTiming {
        self.timing
    }

    /// Returns the submitted result schema.
    #[must_use]
    pub const fn result_schema(&self) -> ActionEvaluationArtifactSchemaId {
        self.result_schema
    }

    /// Returns the exact submitted canonical bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Dispatch-safe runtime projection of one pending evaluator request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingActionEvaluationRaw {
    invocation: ActionEvaluationInvocationId,
    request: ActionEvaluationRequestId,
    implementation: LifecycleImplementationId,
    request_artifact: ActionEvaluationRequestArtifact,
    result_schema: ActionEvaluationArtifactSchemaId,
    admission_mode: DeferredActionAdmissionModeV1,
}

impl PendingActionEvaluationRaw {
    pub(crate) fn from_invocation(record: &ActionEvaluationInvocationRecord) -> Option<Self> {
        if !matches!(
            record.state(),
            ActionEvaluationInvocationState::DispatchPending
        ) {
            return None;
        }
        Some(Self {
            invocation: record.invocation(),
            request: record.request_id()?,
            implementation: record.implementation(),
            request_artifact: record.request()?.clone(),
            result_schema: record.result_schema()?,
            admission_mode: record.admission_mode(),
        })
    }

    /// Returns the logical evaluator invocation.
    #[must_use]
    pub const fn invocation(&self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    /// Returns the exact retained request identity.
    #[must_use]
    pub const fn request(&self) -> ActionEvaluationRequestId {
        self.request
    }

    /// Returns the selected evaluator implementation.
    #[must_use]
    pub const fn implementation(&self) -> LifecycleImplementationId {
        self.implementation
    }

    /// Returns the bounded actor-safe request artifact.
    #[must_use]
    pub const fn request_artifact(&self) -> &ActionEvaluationRequestArtifact {
        &self.request_artifact
    }

    /// Returns the only accepted result schema.
    #[must_use]
    pub const fn result_schema(&self) -> ActionEvaluationArtifactSchemaId {
        self.result_schema
    }

    /// Returns the invocation's fixed result-admission mode.
    #[must_use]
    pub const fn admission_mode(&self) -> DeferredActionAdmissionModeV1 {
        self.admission_mode
    }
}

/// Stable result retained for one action-evaluation capture identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationCaptureOutcome {
    /// Bounded result bytes were retained and scheduled for interpretation.
    ResultCaptured {
        /// Authority record that accepted the capture.
        record: AuthorityRecordId,
        /// Logical invocation that received the result.
        invocation: ActionEvaluationInvocationId,
        /// Semantic identity of the retained result.
        result: ActionEvaluationResultId,
        /// Exact result-interpretation moment.
        effective: SimMoment,
    },
    /// Oversized bytes were reduced to bounded evidence and scheduled for fallback.
    ArtifactRejected {
        /// Authority record that accepted the rejection evidence.
        record: AuthorityRecordId,
        /// Logical invocation whose fallback was scheduled.
        invocation: ActionEvaluationInvocationId,
        /// Bounded evidence for the rejected result bytes.
        failure: ActionEvaluationArtifactFailure,
        /// Exact fallback moment.
        effective: SimMoment,
    },
}

impl ActionEvaluationCaptureOutcome {
    /// Returns the authority record that retained this outcome.
    #[must_use]
    pub const fn record(self) -> AuthorityRecordId {
        match self {
            Self::ResultCaptured { record, .. } | Self::ArtifactRejected { record, .. } => record,
        }
    }

    /// Returns the invocation named by this outcome.
    #[must_use]
    pub const fn invocation(self) -> ActionEvaluationInvocationId {
        match self {
            Self::ResultCaptured { invocation, .. } | Self::ArtifactRejected { invocation, .. } => {
                invocation
            }
        }
    }

    /// Returns the exact simulated delivery or fallback moment.
    #[must_use]
    pub const fn effective(self) -> SimMoment {
        match self {
            Self::ResultCaptured { effective, .. } | Self::ArtifactRejected { effective, .. } => {
                effective
            }
        }
    }
}

/// Closed semantic role of a retained action-evaluation artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionEvaluationArtifactRole {
    /// Actor-safe bytes that may be dispatched to the evaluator.
    Request,
    /// Captured evaluator result bytes.
    Result,
    /// Engine-private candidate resolution and continuation bytes.
    PrivateContinuation,
    /// Engine-private positive dependency witness bytes.
    PrivateReadWitness,
}

impl ActionEvaluationArtifactRole {
    const fn canonical_tag(self) -> u32 {
        match self {
            Self::Request => 0,
            Self::Result => 1,
            Self::PrivateContinuation => 2,
            Self::PrivateReadWitness => 3,
        }
    }
}

/// Bounded evidence for an artifact rejected without retaining its raw bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionEvaluationArtifactFailure {
    role: ActionEvaluationArtifactRole,
    schema: ActionEvaluationArtifactSchemaId,
    actual_length: u64,
    digest: ActionEvaluationArtifactDigest,
}

impl ActionEvaluationArtifactFailure {
    /// Returns the rejected artifact role.
    #[must_use]
    pub const fn role(self) -> ActionEvaluationArtifactRole {
        self.role
    }

    /// Returns the rejected artifact schema.
    #[must_use]
    pub const fn schema(self) -> ActionEvaluationArtifactSchemaId {
        self.schema
    }

    /// Returns the exact rejected byte length.
    #[must_use]
    pub const fn actual_length(self) -> u64 {
        self.actual_length
    }

    /// Returns the role-bound digest of the rejected bytes.
    #[must_use]
    pub const fn digest(self) -> ActionEvaluationArtifactDigest {
        self.digest
    }

    fn write_canonical(self, writer: &mut CanonicalWriter) {
        writer.write_discriminant(self.role.canonical_tag());
        write_fixed(writer, self.schema.as_bytes());
        writer.write_u64(self.actual_length);
        write_fixed(writer, self.digest.as_bytes());
    }
}

/// Why an opaque action-evaluation artifact failed its runtime envelope check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationArtifactError {
    /// Deferred action evaluation is disabled by the execution closure.
    DeferredEvaluationDisabled,
    /// Durable data named a role other than the wrapper's fixed role.
    RoleMismatch {
        /// Role fixed by the concrete wrapper.
        expected: ActionEvaluationArtifactRole,
        /// Role found in durable data.
        actual: ActionEvaluationArtifactRole,
    },
    /// Durable data named a schema other than the invocation's fixed schema.
    SchemaMismatch {
        /// Schema fixed by the invocation binding.
        expected: ActionEvaluationArtifactSchemaId,
        /// Schema found in the artifact.
        actual: ActionEvaluationArtifactSchemaId,
    },
    /// Artifact bytes exceed the role-specific configured maximum.
    LengthExceeded {
        /// Configured maximum byte length.
        maximum: u32,
        /// Digest-complete evidence retained without the raw oversized bytes.
        failure: ActionEvaluationArtifactFailure,
    },
    /// The retained byte length does not describe the exact retained bytes.
    LengthMismatch {
        /// Artifact role whose retained envelope is inconsistent.
        role: ActionEvaluationArtifactRole,
        /// Byte length carried by durable data.
        recorded: u32,
        /// Exact supplied byte length.
        actual: u64,
    },
    /// The retained digest does not cover the exact role, schema, and bytes.
    DigestMismatch {
        /// Digest derived from the retained role, schema, and bytes.
        expected: ActionEvaluationArtifactDigest,
        /// Digest found in durable data.
        actual: ActionEvaluationArtifactDigest,
    },
}

impl fmt::Display for ActionEvaluationArtifactError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeferredEvaluationDisabled => {
                formatter.write_str("deferred action evaluation is disabled")
            }
            Self::RoleMismatch { expected, actual } => {
                write!(
                    formatter,
                    "action-evaluation artifact role {actual:?} does not match {expected:?}"
                )
            }
            Self::SchemaMismatch { expected, actual } => {
                write!(
                    formatter,
                    "action-evaluation artifact schema {actual} does not match {expected}"
                )
            }
            Self::LengthExceeded { maximum, failure } => write!(
                formatter,
                "{:?} artifact length {} exceeds configured maximum {maximum}",
                failure.role(),
                failure.actual_length(),
            ),
            Self::LengthMismatch {
                role,
                recorded,
                actual,
            } => write!(
                formatter,
                "{role:?} artifact records length {recorded} for {actual} supplied bytes"
            ),
            Self::DigestMismatch { expected, actual } => write!(
                formatter,
                "action-evaluation artifact digest {actual} does not match {expected}"
            ),
        }
    }
}

impl std::error::Error for ActionEvaluationArtifactError {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BoundedActionEvaluationArtifact {
    role: ActionEvaluationArtifactRole,
    schema: ActionEvaluationArtifactSchemaId,
    length: u32,
    bytes: Box<[u8]>,
    digest: ActionEvaluationArtifactDigest,
}

impl BoundedActionEvaluationArtifact {
    fn new(
        role: ActionEvaluationArtifactRole,
        schema: ActionEvaluationArtifactSchemaId,
        bytes: Vec<u8>,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationArtifactError> {
        let digest = artifact_digest(role, schema, &bytes);
        let failure = ActionEvaluationArtifactFailure {
            role,
            schema,
            actual_length: bytes.len() as u64,
            digest,
        };
        let length = checked_artifact_length(failure, control)?;
        Ok(Self {
            role,
            schema,
            length,
            bytes: bytes.into_boxed_slice(),
            digest,
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "durable artifact decoding checks every recorded envelope coordinate"
    )]
    fn from_recorded(
        expected_role: ActionEvaluationArtifactRole,
        actual_role: ActionEvaluationArtifactRole,
        expected_schema: ActionEvaluationArtifactSchemaId,
        actual_schema: ActionEvaluationArtifactSchemaId,
        recorded_length: u32,
        bytes: Vec<u8>,
        actual_digest: ActionEvaluationArtifactDigest,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationArtifactError> {
        if actual_role != expected_role {
            return Err(ActionEvaluationArtifactError::RoleMismatch {
                expected: expected_role,
                actual: actual_role,
            });
        }
        if actual_schema != expected_schema {
            return Err(ActionEvaluationArtifactError::SchemaMismatch {
                expected: expected_schema,
                actual: actual_schema,
            });
        }
        let actual_length = bytes.len() as u64;
        if u64::from(recorded_length) != actual_length {
            return Err(ActionEvaluationArtifactError::LengthMismatch {
                role: actual_role,
                recorded: recorded_length,
                actual: actual_length,
            });
        }
        let artifact = Self::new(actual_role, actual_schema, bytes, control)?;
        if artifact.digest != actual_digest {
            return Err(ActionEvaluationArtifactError::DigestMismatch {
                expected: artifact.digest,
                actual: actual_digest,
            });
        }
        Ok(artifact)
    }

    fn verify(
        &self,
        expected_role: ActionEvaluationArtifactRole,
        expected_schema: ActionEvaluationArtifactSchemaId,
        control: DeferredActionControlV1,
    ) -> Result<(), ActionEvaluationArtifactError> {
        let verified = Self::from_recorded(
            expected_role,
            self.role,
            expected_schema,
            self.schema,
            self.length,
            self.bytes.to_vec(),
            self.digest,
            control,
        )?;
        if verified.length == self.length {
            Ok(())
        } else {
            Err(ActionEvaluationArtifactError::LengthMismatch {
                role: self.role,
                recorded: self.length,
                actual: u64::from(verified.length),
            })
        }
    }

    const fn role(&self) -> ActionEvaluationArtifactRole {
        self.role
    }

    const fn schema(&self) -> ActionEvaluationArtifactSchemaId {
        self.schema
    }

    const fn length(&self) -> u32 {
        self.length
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    const fn digest(&self) -> ActionEvaluationArtifactDigest {
        self.digest
    }

    fn write_canonical(&self, writer: &mut CanonicalWriter) {
        writer.write_discriminant(self.role.canonical_tag());
        write_fixed(writer, self.schema.as_bytes());
        writer.write_u32(self.length);
        write_blob(writer, &self.bytes);
        write_fixed(writer, self.digest.as_bytes());
    }
}

macro_rules! action_artifact {
    (
        $(#[$metadata:meta])*
        $name:ident,
        $role:expr
    ) => {
        $(#[$metadata])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name(BoundedActionEvaluationArtifact);

        impl $name {
            /// Retains canonical bytes under the role-specific execution bound.
            pub fn new(
                schema: ActionEvaluationArtifactSchemaId,
                bytes: Vec<u8>,
                control: DeferredActionControlV1,
            ) -> Result<Self, ActionEvaluationArtifactError> {
                BoundedActionEvaluationArtifact::new($role, schema, bytes, control).map(Self)
            }

            /// Rechecks role, schema, configured bound, length, and digest.
            pub fn verify(
                &self,
                expected_schema: ActionEvaluationArtifactSchemaId,
                control: DeferredActionControlV1,
            ) -> Result<(), ActionEvaluationArtifactError> {
                self.0.verify($role, expected_schema, control)
            }

            /// Returns the wrapper's fixed semantic role.
            #[must_use]
            pub const fn role(&self) -> ActionEvaluationArtifactRole {
                self.0.role()
            }

            /// Returns the exact artifact schema identity.
            #[must_use]
            pub const fn schema(&self) -> ActionEvaluationArtifactSchemaId {
                self.0.schema()
            }

            /// Returns the checked byte length.
            #[must_use]
            pub const fn length(&self) -> u32 {
                self.0.length()
            }

            /// Returns the exact retained canonical bytes.
            #[must_use]
            pub fn bytes(&self) -> &[u8] {
                self.0.bytes()
            }

            /// Returns the role-bound artifact digest.
            #[must_use]
            pub const fn digest(&self) -> ActionEvaluationArtifactDigest {
                self.0.digest()
            }

            fn write_canonical(&self, writer: &mut CanonicalWriter) {
                self.0.write_canonical(writer);
            }
        }
    };
}

action_artifact!(
    /// Bounded actor-safe request bytes retained before dispatch.
    ActionEvaluationRequestArtifact,
    ActionEvaluationArtifactRole::Request
);
action_artifact!(
    /// Bounded evaluator result bytes retained before later use.
    ActionEvaluationResultArtifact,
    ActionEvaluationArtifactRole::Result
);
action_artifact!(
    /// Bounded engine-private continuation bytes retained with the request.
    ActionEvaluationPrivateContinuationArtifact,
    ActionEvaluationArtifactRole::PrivateContinuation
);
action_artifact!(
    /// Bounded engine-private positive read-witness bytes.
    ActionEvaluationPrivateReadWitnessArtifact,
    ActionEvaluationArtifactRole::PrivateReadWitness
);

#[cfg(test)]
impl ActionEvaluationRequestArtifact {
    fn from_recorded(
        role: ActionEvaluationArtifactRole,
        expected_schema: ActionEvaluationArtifactSchemaId,
        actual_schema: ActionEvaluationArtifactSchemaId,
        recorded_length: u32,
        bytes: Vec<u8>,
        digest: ActionEvaluationArtifactDigest,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationArtifactError> {
        BoundedActionEvaluationArtifact::from_recorded(
            ActionEvaluationArtifactRole::Request,
            role,
            expected_schema,
            actual_schema,
            recorded_length,
            bytes,
            digest,
            control,
        )
        .map(Self)
    }
}

impl ActionEvaluationRequestId {
    /// Derives the exact request identity from invocation, schema, and digest.
    #[must_use]
    pub fn derive(
        invocation: ActionEvaluationInvocationId,
        request: &ActionEvaluationRequestArtifact,
    ) -> Self {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_REQUEST_ID_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_ARTIFACT_SCHEMA_VERSION);
        write_fixed(&mut writer, invocation.as_bytes());
        write_fixed(&mut writer, request.schema().as_bytes());
        write_fixed(&mut writer, request.digest().as_bytes());
        Self(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }
}

impl ActionEvaluationResultId {
    /// Derives the exact result identity from request, schema, and digest.
    #[must_use]
    pub fn derive(
        request: ActionEvaluationRequestId,
        result: &ActionEvaluationResultArtifact,
    ) -> Self {
        Self::derive_parts(request, result.schema(), result.digest())
    }

    fn derive_parts(
        request: ActionEvaluationRequestId,
        schema: ActionEvaluationArtifactSchemaId,
        digest: ActionEvaluationArtifactDigest,
    ) -> Self {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_RESULT_ID_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_ARTIFACT_SCHEMA_VERSION);
        write_fixed(&mut writer, request.as_bytes());
        write_fixed(&mut writer, schema.as_bytes());
        write_fixed(&mut writer, digest.as_bytes());
        Self(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }
}

impl ActionEvaluationCaptureFingerprint {
    /// Derives a capture-body fingerprint independently of its numeric ID.
    #[must_use]
    pub fn derive(
        invocation: ActionEvaluationInvocationId,
        request: ActionEvaluationRequestId,
        result: ActionEvaluationResultId,
        effective: SimMoment,
        admission_mode: DeferredActionAdmissionModeV1,
        artifact: &ActionEvaluationResultArtifact,
    ) -> Self {
        Self::derive_parts(
            invocation,
            request,
            result,
            effective,
            admission_mode,
            artifact.schema(),
            artifact.digest(),
        )
    }

    fn derive_failure(
        invocation: ActionEvaluationInvocationId,
        request: ActionEvaluationRequestId,
        result: ActionEvaluationResultId,
        effective: SimMoment,
        admission_mode: DeferredActionAdmissionModeV1,
        failure: ActionEvaluationArtifactFailure,
    ) -> Self {
        Self::derive_parts(
            invocation,
            request,
            result,
            effective,
            admission_mode,
            failure.schema(),
            failure.digest(),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the capture fingerprint binds one exact submitted result body"
    )]
    fn derive_parts(
        invocation: ActionEvaluationInvocationId,
        request: ActionEvaluationRequestId,
        result: ActionEvaluationResultId,
        effective: SimMoment,
        admission_mode: DeferredActionAdmissionModeV1,
        schema: ActionEvaluationArtifactSchemaId,
        digest: ActionEvaluationArtifactDigest,
    ) -> Self {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_CAPTURE_FINGERPRINT_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_CAPTURE_SCHEMA_VERSION);
        write_fixed(&mut writer, invocation.as_bytes());
        write_fixed(&mut writer, request.as_bytes());
        write_fixed(&mut writer, result.as_bytes());
        write_moment(&mut writer, effective);
        writer.write_discriminant(admission_mode_tag(admission_mode));
        write_fixed(&mut writer, schema.as_bytes());
        write_fixed(&mut writer, digest.as_bytes());
        Self(ContentDigest::of_canonical(&writer.finish()).into_bytes())
    }
}

/// Actor-visible reason that one logical evaluation was created.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationInvocationCause {
    /// First deferred evaluation for the opportunity.
    Initial,
    /// A predecessor observed a changed actor-visible request.
    VisibleInputChanged {
        /// Terminal predecessor linked to this successor.
        predecessor: ActionEvaluationInvocationId,
    },
}

/// Positive freshness classification of an applied retained result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationResultFreshness {
    /// Policy projection and private execution witness still validate exactly.
    Current,
    /// Trusted projection rebuilt byte-identical actor-visible input.
    ProjectionRebound,
    /// Actor-visible projection stayed current while private legality was rebuilt.
    ExecutionRevalidated,
    /// Projection was rebound byte-identically and private legality was rebuilt.
    ProjectionReboundAndExecutionRevalidated,
}

/// Concrete reason that the fixed later-wake fallback became necessary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ActionEvaluationFallbackCause {
    /// An explicit host cancellation won serialization.
    Cancelled,
    /// An explicit host timeout won serialization.
    TimedOut,
    /// The host reported that evaluation failed.
    HostFailure,
    /// Captured bytes did not decode to a valid decision for the request.
    InvalidResult,
    /// Actor-visible input changed after the configured reinvocation budget.
    VisibleReinvocationExhausted,
    /// A role-bound artifact exceeded its configured retained-byte bound.
    ArtifactRejected(ActionEvaluationArtifactFailure),
}

/// Closed terminal outcome of one logical action evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvaluationTerminal {
    /// The exact captured result was applied after freshness validation.
    Applied {
        /// Applied result identity.
        result: ActionEvaluationResultId,
        /// Positive freshness evidence used for reuse.
        freshness: ActionEvaluationResultFreshness,
    },
    /// Actor-visible input changed and produced a successor invocation.
    Reinvoked {
        /// Discarded predecessor result.
        result: ActionEvaluationResultId,
        /// New logical evaluation identity.
        successor: ActionEvaluationInvocationId,
    },
    /// A concrete failure completed its later fallback.
    Failed {
        /// Failure that required fallback.
        cause: ActionEvaluationFallbackCause,
    },
}

/// Closed durable state of one action-policy evaluation invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionEvaluationInvocationState {
    /// The exact request is committed and externally dispatchable.
    DispatchPending,
    /// An exact result is captured and scheduled for later interpretation.
    ResultCaptured {
        /// Semantic result identity.
        result: ActionEvaluationResultId,
        /// Retained bounded result artifact.
        artifact: ActionEvaluationResultArtifact,
        /// Capture namespace identity.
        capture: ActionEvaluationCaptureId,
        /// Exact capture request fingerprint.
        capture_fingerprint: ActionEvaluationCaptureFingerprint,
        /// Effective simulation moment.
        effective: SimMoment,
        /// Exact installed `ResultReady` scheduler coordinate.
        scheduler_key: SchedulerKey,
    },
    /// A fixed failure disposition is scheduled as later ordinary work.
    FallbackPending {
        /// Concrete cause of fallback.
        cause: ActionEvaluationFallbackCause,
        /// Exact installed fallback scheduler coordinate.
        scheduler_key: SchedulerKey,
    },
    /// The invocation can never affect the opportunity again.
    Terminal(ActionEvaluationTerminal),
}

/// Why an invocation record or checked transition is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionEvaluationInvocationError {
    /// One retained artifact failed its exact envelope validation.
    Artifact(ActionEvaluationArtifactError),
    /// Deferred evaluation is disabled.
    DeferredEvaluationDisabled,
    /// Result artifacts can fail only during capture, never while opening an invocation.
    InvocationOpeningArtifactRoleMismatch {
        /// Artifact role supplied by the rejected opening.
        actual: ActionEvaluationArtifactRole,
    },
    /// Rejection evidence does not exceed the invocation's configured role bound.
    RejectedArtifactWithinBound {
        /// Configured maximum byte length for the artifact role.
        maximum: u32,
        /// Supplied evidence that cannot justify a rejection under that bound.
        failure: ActionEvaluationArtifactFailure,
    },
    /// Result-artifact rejection must enter through the capture protocol.
    ArtifactRejectionRequiresCapture,
    /// Host cancellation, timeout, and failure must enter through management.
    ManagementFallbackRequiresManagement,
    /// The retained invocation does not match its actor-visible derivation inputs.
    InvocationIdentityMismatch {
        /// Invocation derived from the retained opportunity, generation, and fingerprints.
        expected: ActionEvaluationInvocationId,
        /// Invocation supplied by the enclosing transition.
        actual: ActionEvaluationInvocationId,
    },
    /// The waiting version is not the checked successor of the open version.
    OpportunityVersionDiscontinuity {
        /// Version before entering the waiting state.
        before: ActionOpportunityVersion,
        /// Retained waiting version.
        waiting: ActionOpportunityVersion,
    },
    /// Blocking frontier presence does not match the configured admission mode.
    BlockingFrontierMismatch {
        /// Configured admission mode.
        admission_mode: DeferredActionAdmissionModeV1,
        /// Retained blocking frontier.
        blocked_at_frontier: Option<SimMoment>,
    },
    /// A blocking frontier must be a later simulation coordinate than creation.
    BlockingFrontierNotLater {
        /// Invocation creation moment.
        creation: SimMoment,
        /// Retained blocking frontier.
        blocked_at_frontier: SimMoment,
    },
    /// Caller named a waiting opportunity version other than the retained one.
    StaleWaitingVersion {
        /// Waiting version named by the caller.
        expected: ActionOpportunityVersion,
        /// Waiting version retained by the invocation.
        actual: ActionOpportunityVersion,
    },
    /// Invocation state does not permit the requested transition.
    StateMismatch,
    /// Captured result does not use the invocation's fixed result schema.
    ResultSchemaMismatch {
        /// Result schema fixed at invocation creation.
        expected: ActionEvaluationArtifactSchemaId,
        /// Schema carried by the result artifact.
        actual: ActionEvaluationArtifactSchemaId,
    },
    /// Capture fingerprint does not bind the exact result body.
    CaptureFingerprintMismatch {
        /// Fingerprint derived from exact capture content.
        expected: ActionEvaluationCaptureFingerprint,
        /// Supplied capture fingerprint.
        actual: ActionEvaluationCaptureFingerprint,
    },
    /// Scheduled action-evaluation work has the wrong lane or moment.
    SchedulerBindingMismatch,
    /// Effective result time violates the selected admission mode.
    EffectiveMomentMismatch,
    /// Terminal result identity is not the retained captured result.
    ResultIdentityMismatch {
        /// Retained captured result.
        expected: ActionEvaluationResultId,
        /// Result named by the transition.
        actual: ActionEvaluationResultId,
    },
    /// Visible reinvocation was requested after its budget was exhausted.
    ReinvocationBudgetExhausted,
    /// A linked reinvocation did not advance the actor-visible generation exactly once.
    EvaluationGenerationDiscontinuity {
        /// Predecessor generation.
        predecessor: ActionEvaluationGeneration,
        /// Proposed successor generation.
        successor: ActionEvaluationGeneration,
    },
    /// A successor was built from a record that did not terminally name it.
    PredecessorNotReinvoked,
    /// Successor execution control differs from its predecessor's fixed control.
    ReinvocationControlMismatch,
    /// A successor reused the predecessor's logical invocation identity.
    SuccessorIdentityReused,
}

impl From<ActionEvaluationArtifactError> for ActionEvaluationInvocationError {
    fn from(error: ActionEvaluationArtifactError) -> Self {
        Self::Artifact(error)
    }
}

/// Complete bounded artifacts required for evaluator dispatch and result use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationDispatchPayload {
    request: ActionEvaluationRequestArtifact,
    request_id: ActionEvaluationRequestId,
    result_schema: ActionEvaluationArtifactSchemaId,
    private_continuation: ActionEvaluationPrivateContinuationArtifact,
    private_read_witness: ActionEvaluationPrivateReadWitnessArtifact,
}

/// Closed retained artifact payload for one evaluation invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionEvaluationInvocationPayload {
    /// All dispatchable artifacts, grouped behind one narrow retained allocation.
    Dispatchable(Box<ActionEvaluationDispatchPayload>),
    /// One oversize artifact failed before raw bytes could be retained.
    ArtifactRejected {
        /// Digest-complete rejection evidence.
        failure: ActionEvaluationArtifactFailure,
    },
}

/// Self-contained retained state for one deferred action evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionEvaluationInvocationRecord {
    invocation: ActionEvaluationInvocationId,
    opportunity: ActionOpportunityId,
    pre_wait_version: ActionOpportunityVersion,
    waiting_version: ActionOpportunityVersion,
    evaluation_generation: ActionEvaluationGeneration,
    policy_semantics: [u8; 32],
    action_input_fingerprint: [u8; 32],
    cause: ActionEvaluationInvocationCause,
    implementation: LifecycleImplementationId,
    payload: ActionEvaluationInvocationPayload,
    admission_mode: DeferredActionAdmissionModeV1,
    remaining_visible_reinvocations: u32,
    fallback: DeferredActionFallbackV1,
    creation_moment: SimMoment,
    source_cursor: AuthorityCursor,
    blocked_at_frontier: Option<SimMoment>,
    state: ActionEvaluationInvocationState,
}

#[derive(Clone, Copy)]
struct CheckedInvocationControl {
    admission_mode: DeferredActionAdmissionModeV1,
    remaining_visible_reinvocations: u32,
    fallback: DeferredActionFallbackV1,
}

impl ActionEvaluationInvocationRecord {
    /// Constructs the first committed dispatch obligation for an opportunity.
    #[allow(
        clippy::too_many_arguments,
        reason = "the invocation record deliberately retains every exact artifact and authority binding"
    )]
    pub(crate) fn dispatch_pending(
        invocation: ActionEvaluationInvocationId,
        opportunity: ActionOpportunityId,
        pre_wait_version: ActionOpportunityVersion,
        waiting_version: ActionOpportunityVersion,
        evaluation_generation: ActionEvaluationGeneration,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
        implementation: LifecycleImplementationId,
        request: ActionEvaluationRequestArtifact,
        result_schema: ActionEvaluationArtifactSchemaId,
        private_continuation: ActionEvaluationPrivateContinuationArtifact,
        private_read_witness: ActionEvaluationPrivateReadWitnessArtifact,
        creation_moment: SimMoment,
        source_cursor: AuthorityCursor,
        blocked_at_frontier: Option<SimMoment>,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        let checked = checked_invocation_control(
            invocation,
            opportunity,
            pre_wait_version,
            waiting_version,
            evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
            creation_moment,
            blocked_at_frontier,
            control,
        )?;
        request.verify(request.schema(), control)?;
        private_continuation.verify(private_continuation.schema(), control)?;
        private_read_witness.verify(private_read_witness.schema(), control)?;
        let request_id = ActionEvaluationRequestId::derive(invocation, &request);
        Ok(Self {
            invocation,
            opportunity,
            pre_wait_version,
            waiting_version,
            evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
            cause: ActionEvaluationInvocationCause::Initial,
            implementation,
            payload: ActionEvaluationInvocationPayload::Dispatchable(Box::new(
                ActionEvaluationDispatchPayload {
                    request,
                    request_id,
                    result_schema,
                    private_continuation,
                    private_read_witness,
                },
            )),
            admission_mode: checked.admission_mode,
            remaining_visible_reinvocations: checked.remaining_visible_reinvocations,
            fallback: checked.fallback,
            creation_moment,
            source_cursor,
            blocked_at_frontier,
            state: ActionEvaluationInvocationState::DispatchPending,
        })
    }

    /// Constructs a non-dispatchable invocation whose bounded artifact failed.
    #[allow(
        clippy::too_many_arguments,
        reason = "artifact rejection retains one complete invocation and later-wake binding"
    )]
    pub(crate) fn artifact_rejected(
        invocation: ActionEvaluationInvocationId,
        opportunity: ActionOpportunityId,
        pre_wait_version: ActionOpportunityVersion,
        waiting_version: ActionOpportunityVersion,
        evaluation_generation: ActionEvaluationGeneration,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
        implementation: LifecycleImplementationId,
        failure: ActionEvaluationArtifactFailure,
        creation_moment: SimMoment,
        source_cursor: AuthorityCursor,
        blocked_at_frontier: Option<SimMoment>,
        scheduler_key: SchedulerKey,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        let checked = checked_invocation_control(
            invocation,
            opportunity,
            pre_wait_version,
            waiting_version,
            evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
            creation_moment,
            blocked_at_frontier,
            control,
        )?;
        check_rejected_invocation_artifact(failure, None, control)?;
        if scheduler_key.lane() != SchedulerLaneV2::ActionEvaluation
            || scheduler_key.moment() <= creation_moment
            || matches!(
                checked.admission_mode,
                DeferredActionAdmissionModeV1::FrontierBlocking
            ) && Some(scheduler_key.moment()) != blocked_at_frontier
        {
            return Err(ActionEvaluationInvocationError::SchedulerBindingMismatch);
        }
        Ok(Self {
            invocation,
            opportunity,
            pre_wait_version,
            waiting_version,
            evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
            cause: ActionEvaluationInvocationCause::Initial,
            implementation,
            payload: ActionEvaluationInvocationPayload::ArtifactRejected { failure },
            admission_mode: checked.admission_mode,
            remaining_visible_reinvocations: checked.remaining_visible_reinvocations,
            fallback: checked.fallback,
            creation_moment,
            source_cursor,
            blocked_at_frontier,
            state: ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(failure),
                scheduler_key,
            },
        })
    }

    /// Constructs a linked dispatch after actor-visible input changed.
    #[allow(
        clippy::too_many_arguments,
        reason = "a linked reinvocation retains one complete replacement artifact binding"
    )]
    pub(crate) fn visible_reinvocation_dispatch_pending(
        predecessor: &Self,
        invocation: ActionEvaluationInvocationId,
        pre_wait_version: ActionOpportunityVersion,
        waiting_version: ActionOpportunityVersion,
        evaluation_generation: ActionEvaluationGeneration,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
        request: ActionEvaluationRequestArtifact,
        private_continuation: ActionEvaluationPrivateContinuationArtifact,
        private_read_witness: ActionEvaluationPrivateReadWitnessArtifact,
        creation_moment: SimMoment,
        source_cursor: AuthorityCursor,
        blocked_at_frontier: Option<SimMoment>,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        let ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Reinvoked {
            successor,
            ..
        }) = predecessor.state
        else {
            return Err(ActionEvaluationInvocationError::PredecessorNotReinvoked);
        };
        if successor != invocation {
            return Err(ActionEvaluationInvocationError::PredecessorNotReinvoked);
        }
        let Some(expected_generation) = predecessor.evaluation_generation.checked_next() else {
            return Err(
                ActionEvaluationInvocationError::EvaluationGenerationDiscontinuity {
                    predecessor: predecessor.evaluation_generation,
                    successor: evaluation_generation,
                },
            );
        };
        if evaluation_generation != expected_generation {
            return Err(
                ActionEvaluationInvocationError::EvaluationGenerationDiscontinuity {
                    predecessor: predecessor.evaluation_generation,
                    successor: evaluation_generation,
                },
            );
        }
        let Some(remaining_visible_reinvocations) =
            predecessor.remaining_visible_reinvocations.checked_sub(1)
        else {
            return Err(ActionEvaluationInvocationError::ReinvocationBudgetExhausted);
        };
        if policy_semantics != predecessor.policy_semantics
            || control.admission_mode() != Some(predecessor.admission_mode)
            || control.fallback() != Some(predecessor.fallback)
        {
            return Err(ActionEvaluationInvocationError::ReinvocationControlMismatch);
        }
        let predecessor_payload = match &predecessor.payload {
            ActionEvaluationInvocationPayload::Dispatchable(payload) => payload,
            ActionEvaluationInvocationPayload::ArtifactRejected { .. } => {
                return Err(ActionEvaluationInvocationError::PredecessorNotReinvoked);
            }
        };
        check_artifact_schema(predecessor_payload.request.schema(), request.schema())?;
        check_artifact_schema(
            predecessor_payload.private_continuation.schema(),
            private_continuation.schema(),
        )?;
        check_artifact_schema(
            predecessor_payload.private_read_witness.schema(),
            private_read_witness.schema(),
        )?;
        let mut successor_record = Self::dispatch_pending(
            invocation,
            predecessor.opportunity,
            pre_wait_version,
            waiting_version,
            evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
            predecessor.implementation,
            request,
            predecessor_payload.result_schema,
            private_continuation,
            private_read_witness,
            creation_moment,
            source_cursor,
            blocked_at_frontier,
            control,
        )?;
        successor_record.cause = ActionEvaluationInvocationCause::VisibleInputChanged {
            predecessor: predecessor.invocation,
        };
        successor_record.remaining_visible_reinvocations = remaining_visible_reinvocations;
        Ok(successor_record)
    }

    /// Constructs a linked reinvocation whose replacement artifacts exceeded
    /// their retained bound and therefore proceed directly to later fallback.
    #[allow(
        clippy::too_many_arguments,
        reason = "a linked rejected reinvocation retains its complete identity and fallback binding"
    )]
    pub(crate) fn visible_reinvocation_artifact_rejected(
        predecessor: &Self,
        invocation: ActionEvaluationInvocationId,
        pre_wait_version: ActionOpportunityVersion,
        waiting_version: ActionOpportunityVersion,
        evaluation_generation: ActionEvaluationGeneration,
        policy_semantics: [u8; 32],
        action_input_fingerprint: [u8; 32],
        failure: ActionEvaluationArtifactFailure,
        creation_moment: SimMoment,
        source_cursor: AuthorityCursor,
        blocked_at_frontier: Option<SimMoment>,
        scheduler_key: SchedulerKey,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        let ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Reinvoked {
            successor,
            ..
        }) = predecessor.state
        else {
            return Err(ActionEvaluationInvocationError::PredecessorNotReinvoked);
        };
        if successor != invocation {
            return Err(ActionEvaluationInvocationError::PredecessorNotReinvoked);
        }
        let Some(expected_generation) = predecessor.evaluation_generation.checked_next() else {
            return Err(
                ActionEvaluationInvocationError::EvaluationGenerationDiscontinuity {
                    predecessor: predecessor.evaluation_generation,
                    successor: evaluation_generation,
                },
            );
        };
        if evaluation_generation != expected_generation {
            return Err(
                ActionEvaluationInvocationError::EvaluationGenerationDiscontinuity {
                    predecessor: predecessor.evaluation_generation,
                    successor: evaluation_generation,
                },
            );
        }
        let Some(remaining_visible_reinvocations) =
            predecessor.remaining_visible_reinvocations.checked_sub(1)
        else {
            return Err(ActionEvaluationInvocationError::ReinvocationBudgetExhausted);
        };
        let ActionEvaluationInvocationPayload::Dispatchable(payload) = &predecessor.payload else {
            return Err(ActionEvaluationInvocationError::ReinvocationControlMismatch);
        };
        let expected_failure_schema = match failure.role() {
            ActionEvaluationArtifactRole::Request => payload.request.schema(),
            ActionEvaluationArtifactRole::Result => payload.result_schema,
            ActionEvaluationArtifactRole::PrivateContinuation => {
                payload.private_continuation.schema()
            }
            ActionEvaluationArtifactRole::PrivateReadWitness => {
                payload.private_read_witness.schema()
            }
        };
        check_rejected_invocation_artifact(failure, Some(expected_failure_schema), control)?;
        if policy_semantics != predecessor.policy_semantics
            || control.admission_mode() != Some(predecessor.admission_mode)
            || control.fallback() != Some(predecessor.fallback)
        {
            return Err(ActionEvaluationInvocationError::ReinvocationControlMismatch);
        }
        let mut successor_record = Self::artifact_rejected(
            invocation,
            predecessor.opportunity,
            pre_wait_version,
            waiting_version,
            evaluation_generation,
            policy_semantics,
            action_input_fingerprint,
            predecessor.implementation,
            failure,
            creation_moment,
            source_cursor,
            blocked_at_frontier,
            scheduler_key,
            control,
        )?;
        successor_record.cause = ActionEvaluationInvocationCause::VisibleInputChanged {
            predecessor: predecessor.invocation,
        };
        successor_record.remaining_visible_reinvocations = remaining_visible_reinvocations;
        Ok(successor_record)
    }

    /// Returns the logical invocation identity.
    #[must_use]
    pub const fn invocation(&self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    /// Returns the owned action opportunity.
    #[must_use]
    pub const fn opportunity(&self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the open opportunity version before waiting began.
    #[must_use]
    pub const fn pre_wait_version(&self) -> ActionOpportunityVersion {
        self.pre_wait_version
    }

    /// Returns the exact version that must still be waiting.
    #[must_use]
    pub const fn waiting_version(&self) -> ActionOpportunityVersion {
        self.waiting_version
    }

    /// Returns the actor-visible evaluation generation.
    #[must_use]
    pub const fn evaluation_generation(&self) -> ActionEvaluationGeneration {
        self.evaluation_generation
    }

    /// Returns the selected policy-semantics identity input.
    #[must_use]
    pub const fn policy_semantics(&self) -> &[u8; 32] {
        &self.policy_semantics
    }

    /// Returns the complete actor-visible action-input fingerprint.
    #[must_use]
    pub const fn action_input_fingerprint(&self) -> &[u8; 32] {
        &self.action_input_fingerprint
    }

    /// Returns the invocation's actor-visible creation cause.
    #[must_use]
    pub const fn cause(&self) -> ActionEvaluationInvocationCause {
        self.cause
    }

    /// Returns the exact selected action-policy implementation.
    #[must_use]
    pub const fn implementation(&self) -> LifecycleImplementationId {
        self.implementation
    }

    /// Returns the closed retained artifact payload.
    #[must_use]
    pub const fn payload(&self) -> &ActionEvaluationInvocationPayload {
        &self.payload
    }

    /// Returns the retained dispatch-safe request when dispatchable.
    #[must_use]
    pub const fn request(&self) -> Option<&ActionEvaluationRequestArtifact> {
        match &self.payload {
            ActionEvaluationInvocationPayload::Dispatchable(payload) => Some(&payload.request),
            ActionEvaluationInvocationPayload::ArtifactRejected { .. } => None,
        }
    }

    /// Returns the derived request identity when dispatchable.
    #[must_use]
    pub const fn request_id(&self) -> Option<ActionEvaluationRequestId> {
        match &self.payload {
            ActionEvaluationInvocationPayload::Dispatchable(payload) => Some(payload.request_id),
            ActionEvaluationInvocationPayload::ArtifactRejected { .. } => None,
        }
    }

    /// Returns the only result schema accepted by a dispatchable invocation.
    #[must_use]
    pub const fn result_schema(&self) -> Option<ActionEvaluationArtifactSchemaId> {
        match &self.payload {
            ActionEvaluationInvocationPayload::Dispatchable(payload) => Some(payload.result_schema),
            ActionEvaluationInvocationPayload::ArtifactRejected { .. } => None,
        }
    }

    /// Returns the engine-private continuation artifact when dispatchable.
    #[must_use]
    pub const fn private_continuation(
        &self,
    ) -> Option<&ActionEvaluationPrivateContinuationArtifact> {
        match &self.payload {
            ActionEvaluationInvocationPayload::Dispatchable(payload) => {
                Some(&payload.private_continuation)
            }
            ActionEvaluationInvocationPayload::ArtifactRejected { .. } => None,
        }
    }

    /// Returns the engine-private positive read witness when dispatchable.
    #[must_use]
    pub const fn private_read_witness(
        &self,
    ) -> Option<&ActionEvaluationPrivateReadWitnessArtifact> {
        match &self.payload {
            ActionEvaluationInvocationPayload::Dispatchable(payload) => {
                Some(&payload.private_read_witness)
            }
            ActionEvaluationInvocationPayload::ArtifactRejected { .. } => None,
        }
    }

    /// Returns the fixed result-admission mode.
    #[must_use]
    pub const fn admission_mode(&self) -> DeferredActionAdmissionModeV1 {
        self.admission_mode
    }

    /// Returns the remaining actor-visible reinvocation budget.
    #[must_use]
    pub const fn remaining_visible_reinvocations(&self) -> u32 {
        self.remaining_visible_reinvocations
    }

    /// Returns the fixed later-wake fallback.
    #[must_use]
    pub const fn fallback(&self) -> DeferredActionFallbackV1 {
        self.fallback
    }

    /// Returns the creating simulation moment.
    #[must_use]
    pub const fn creation_moment(&self) -> SimMoment {
        self.creation_moment
    }

    /// Returns private authority provenance for the retained projection.
    #[must_use]
    pub const fn source_cursor(&self) -> AuthorityCursor {
        self.source_cursor
    }

    /// Returns the global blocking frontier when configured.
    #[must_use]
    pub const fn blocked_at_frontier(&self) -> Option<SimMoment> {
        self.blocked_at_frontier
    }

    /// Returns the current closed invocation state.
    #[must_use]
    pub const fn state(&self) -> &ActionEvaluationInvocationState {
        &self.state
    }

    /// Returns the canonical digest of the complete retained record.
    #[must_use]
    pub fn digest(&self) -> ActionEvaluationInvocationDigest {
        ActionEvaluationInvocationDigest(
            ContentDigest::of_canonical(&self.canonical_bytes()).into_bytes(),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "capture identity and scheduler binding are one atomic transition body"
    )]
    fn capture_result(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        capture: ActionEvaluationCaptureId,
        capture_fingerprint: ActionEvaluationCaptureFingerprint,
        artifact: ActionEvaluationResultArtifact,
        effective: SimMoment,
        scheduler_key: SchedulerKey,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        self.require_waiting_version(expected_waiting_version)?;
        if !matches!(self.state, ActionEvaluationInvocationState::DispatchPending) {
            return Err(ActionEvaluationInvocationError::StateMismatch);
        }
        let ActionEvaluationInvocationPayload::Dispatchable(payload) = &self.payload else {
            return Err(ActionEvaluationInvocationError::StateMismatch);
        };
        artifact.verify(payload.result_schema, control)?;
        if artifact.schema() != payload.result_schema {
            return Err(ActionEvaluationInvocationError::ResultSchemaMismatch {
                expected: payload.result_schema,
                actual: artifact.schema(),
            });
        }
        if scheduler_key.lane() != SchedulerLaneV2::ActionEvaluation
            || scheduler_key.moment() != effective
        {
            return Err(ActionEvaluationInvocationError::SchedulerBindingMismatch);
        }
        match self.admission_mode {
            DeferredActionAdmissionModeV1::FrontierBlocking
                if self.blocked_at_frontier != Some(effective) =>
            {
                return Err(ActionEvaluationInvocationError::EffectiveMomentMismatch);
            }
            DeferredActionAdmissionModeV1::HostScheduled if effective <= self.creation_moment => {
                return Err(ActionEvaluationInvocationError::EffectiveMomentMismatch);
            }
            DeferredActionAdmissionModeV1::FrontierBlocking
            | DeferredActionAdmissionModeV1::HostScheduled => {}
        }
        let result = ActionEvaluationResultId::derive(payload.request_id, &artifact);
        let expected_fingerprint = ActionEvaluationCaptureFingerprint::derive(
            self.invocation,
            payload.request_id,
            result,
            effective,
            self.admission_mode,
            &artifact,
        );
        if capture_fingerprint != expected_fingerprint {
            return Err(
                ActionEvaluationInvocationError::CaptureFingerprintMismatch {
                    expected: expected_fingerprint,
                    actual: capture_fingerprint,
                },
            );
        }
        Ok(
            self.successor(ActionEvaluationInvocationState::ResultCaptured {
                result,
                artifact,
                capture,
                capture_fingerprint,
                effective,
                scheduler_key,
            }),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "rejected capture identity and scheduler binding are one atomic transition body"
    )]
    fn capture_artifact_rejection(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        capture_fingerprint: ActionEvaluationCaptureFingerprint,
        failure: ActionEvaluationArtifactFailure,
        effective: SimMoment,
        scheduler_key: SchedulerKey,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        self.require_waiting_version(expected_waiting_version)?;
        if !matches!(self.state, ActionEvaluationInvocationState::DispatchPending) {
            return Err(ActionEvaluationInvocationError::StateMismatch);
        }
        let ActionEvaluationInvocationPayload::Dispatchable(payload) = &self.payload else {
            return Err(ActionEvaluationInvocationError::StateMismatch);
        };
        check_rejected_artifact(
            failure,
            ActionEvaluationArtifactRole::Result,
            Some(payload.result_schema),
            control,
        )?;
        if scheduler_key.lane() != SchedulerLaneV2::ActionEvaluation
            || scheduler_key.moment() != effective
        {
            return Err(ActionEvaluationInvocationError::SchedulerBindingMismatch);
        }
        match self.admission_mode {
            DeferredActionAdmissionModeV1::FrontierBlocking
                if self.blocked_at_frontier != Some(effective) =>
            {
                return Err(ActionEvaluationInvocationError::EffectiveMomentMismatch);
            }
            DeferredActionAdmissionModeV1::HostScheduled if effective <= self.creation_moment => {
                return Err(ActionEvaluationInvocationError::EffectiveMomentMismatch);
            }
            DeferredActionAdmissionModeV1::FrontierBlocking
            | DeferredActionAdmissionModeV1::HostScheduled => {}
        }
        let result = ActionEvaluationResultId::derive_parts(
            payload.request_id,
            failure.schema(),
            failure.digest(),
        );
        let expected_fingerprint = ActionEvaluationCaptureFingerprint::derive_failure(
            self.invocation,
            payload.request_id,
            result,
            effective,
            self.admission_mode,
            failure,
        );
        if capture_fingerprint != expected_fingerprint {
            return Err(
                ActionEvaluationInvocationError::CaptureFingerprintMismatch {
                    expected: expected_fingerprint,
                    actual: capture_fingerprint,
                },
            );
        }
        Ok(
            self.successor(ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(failure),
                scheduler_key,
            }),
        )
    }

    fn begin_fallback(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        cause: ActionEvaluationFallbackCause,
        scheduler_key: SchedulerKey,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        if matches!(cause, ActionEvaluationFallbackCause::ArtifactRejected(_)) {
            return Err(ActionEvaluationInvocationError::ArtifactRejectionRequiresCapture);
        }
        if matches!(
            cause,
            ActionEvaluationFallbackCause::Cancelled
                | ActionEvaluationFallbackCause::TimedOut
                | ActionEvaluationFallbackCause::HostFailure
        ) {
            return Err(ActionEvaluationInvocationError::ManagementFallbackRequiresManagement);
        }
        self.require_waiting_version(expected_waiting_version)?;
        let causal_source = match self.state {
            ActionEvaluationInvocationState::DispatchPending => self.creation_moment,
            ActionEvaluationInvocationState::ResultCaptured { effective, .. } => effective,
            ActionEvaluationInvocationState::FallbackPending { .. }
            | ActionEvaluationInvocationState::Terminal(_) => {
                return Err(ActionEvaluationInvocationError::StateMismatch);
            }
        };
        if scheduler_key.lane() != SchedulerLaneV2::ActionEvaluation
            || scheduler_key.moment() <= causal_source
        {
            return Err(ActionEvaluationInvocationError::SchedulerBindingMismatch);
        }
        Ok(
            self.successor(ActionEvaluationInvocationState::FallbackPending {
                cause,
                scheduler_key,
            }),
        )
    }

    fn begin_managed_fallback(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        cause: ActionEvaluationFallbackCause,
        scheduler_key: SchedulerKey,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        if !matches!(
            cause,
            ActionEvaluationFallbackCause::Cancelled
                | ActionEvaluationFallbackCause::TimedOut
                | ActionEvaluationFallbackCause::HostFailure
        ) {
            return Err(ActionEvaluationInvocationError::ManagementFallbackRequiresManagement);
        }
        self.require_waiting_version(expected_waiting_version)?;
        let earliest = match self.state {
            ActionEvaluationInvocationState::DispatchPending => self.creation_moment,
            ActionEvaluationInvocationState::ResultCaptured { effective, .. } => effective,
            ActionEvaluationInvocationState::FallbackPending { .. }
            | ActionEvaluationInvocationState::Terminal(_) => {
                return Err(ActionEvaluationInvocationError::StateMismatch);
            }
        };
        if scheduler_key.lane() != SchedulerLaneV2::ActionEvaluation
            || scheduler_key.moment() < earliest
            || scheduler_key.moment() <= self.creation_moment
        {
            return Err(ActionEvaluationInvocationError::SchedulerBindingMismatch);
        }
        Ok(
            self.successor(ActionEvaluationInvocationState::FallbackPending {
                cause,
                scheduler_key,
            }),
        )
    }

    fn finish_applied(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        result: ActionEvaluationResultId,
        freshness: ActionEvaluationResultFreshness,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        self.require_result(expected_waiting_version, result)?;
        Ok(self.successor(ActionEvaluationInvocationState::Terminal(
            ActionEvaluationTerminal::Applied { result, freshness },
        )))
    }

    fn finish_reinvoked(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        result: ActionEvaluationResultId,
        successor: ActionEvaluationInvocationId,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        self.require_result(expected_waiting_version, result)?;
        if self.remaining_visible_reinvocations == 0 {
            return Err(ActionEvaluationInvocationError::ReinvocationBudgetExhausted);
        }
        if successor == self.invocation {
            return Err(ActionEvaluationInvocationError::SuccessorIdentityReused);
        }
        Ok(self.successor(ActionEvaluationInvocationState::Terminal(
            ActionEvaluationTerminal::Reinvoked { result, successor },
        )))
    }

    fn finish_fallback(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
    ) -> Result<Self, ActionEvaluationInvocationError> {
        self.require_waiting_version(expected_waiting_version)?;
        let ActionEvaluationInvocationState::FallbackPending { cause, .. } = self.state else {
            return Err(ActionEvaluationInvocationError::StateMismatch);
        };
        Ok(self.successor(ActionEvaluationInvocationState::Terminal(
            ActionEvaluationTerminal::Failed { cause },
        )))
    }

    fn require_result(
        &self,
        expected_waiting_version: ActionOpportunityVersion,
        result: ActionEvaluationResultId,
    ) -> Result<(), ActionEvaluationInvocationError> {
        self.require_waiting_version(expected_waiting_version)?;
        let ActionEvaluationInvocationState::ResultCaptured {
            result: retained, ..
        } = self.state
        else {
            return Err(ActionEvaluationInvocationError::StateMismatch);
        };
        if retained == result {
            Ok(())
        } else {
            Err(ActionEvaluationInvocationError::ResultIdentityMismatch {
                expected: retained,
                actual: result,
            })
        }
    }

    fn require_waiting_version(
        &self,
        expected: ActionOpportunityVersion,
    ) -> Result<(), ActionEvaluationInvocationError> {
        if expected == self.waiting_version {
            Ok(())
        } else {
            Err(ActionEvaluationInvocationError::StaleWaitingVersion {
                expected,
                actual: self.waiting_version,
            })
        }
    }

    fn successor(&self, state: ActionEvaluationInvocationState) -> Self {
        let mut successor = self.clone();
        successor.state = state;
        successor
    }

    fn canonical_bytes(&self) -> CanonicalBytes {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_INVOCATION_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_INVOCATION_SCHEMA_VERSION);
        write_fixed(&mut writer, self.invocation.as_bytes());
        write_fixed(&mut writer, self.opportunity.as_bytes());
        writer.write_u64(self.pre_wait_version.get());
        writer.write_u64(self.waiting_version.get());
        writer.write_u64(self.evaluation_generation.get());
        write_fixed(&mut writer, &self.policy_semantics);
        write_fixed(&mut writer, &self.action_input_fingerprint);
        write_invocation_cause(&mut writer, self.cause);
        write_fixed(&mut writer, self.implementation.as_bytes());
        write_invocation_payload(&mut writer, &self.payload);
        writer.write_discriminant(admission_mode_tag(self.admission_mode));
        writer.write_u32(self.remaining_visible_reinvocations);
        writer.write_discriminant(fallback_tag(self.fallback));
        write_moment(&mut writer, self.creation_moment);
        write_blob(&mut writer, self.source_cursor.canonical_bytes().as_bytes());
        write_optional_moment(&mut writer, self.blocked_at_frontier);
        write_invocation_state(&mut writer, &self.state);
        writer.finish()
    }
}

/// Exact-version transition ledger for retained action evaluations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ActionEvaluationInvocationLedger {
    entries: BTreeMap<ActionEvaluationInvocationId, ActionEvaluationInvocationRecord>,
}

/// Why a checked invocation-ledger operation was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActionEvaluationInvocationLedgerError {
    DuplicateInvocation {
        invocation: ActionEvaluationInvocationId,
    },
    UnknownInvocation {
        invocation: ActionEvaluationInvocationId,
    },
    OpportunityMismatch {
        invocation: ActionEvaluationInvocationId,
        opportunity: ActionOpportunityId,
    },
    TransitionSourceMismatch {
        invocation: ActionEvaluationInvocationId,
        expected: ActionEvaluationInvocationDigest,
        actual: ActionEvaluationInvocationDigest,
    },
    Transition(ActionEvaluationInvocationError),
}

impl From<ActionEvaluationInvocationError> for ActionEvaluationInvocationLedgerError {
    fn from(error: ActionEvaluationInvocationError) -> Self {
        Self::Transition(error)
    }
}

impl ActionEvaluationInvocationLedger {
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub(crate) fn get(
        &self,
        invocation: ActionEvaluationInvocationId,
    ) -> Option<&ActionEvaluationInvocationRecord> {
        self.entries.get(&invocation)
    }

    /// Iterates dispatchable obligations in canonical invocation order.
    pub(crate) fn pending_dispatches(
        &self,
    ) -> impl Iterator<Item = &ActionEvaluationInvocationRecord> {
        self.entries.values().filter(|record| {
            matches!(
                record.state(),
                ActionEvaluationInvocationState::DispatchPending
            ) && matches!(
                record.payload(),
                ActionEvaluationInvocationPayload::Dispatchable(_)
            )
        })
    }

    /// Copies every actor-safe dispatch obligation in canonical invocation order.
    #[must_use]
    pub(crate) fn pending_raw(&self) -> Vec<PendingActionEvaluationRaw> {
        self.pending_dispatches()
            .map(|record| {
                PendingActionEvaluationRaw::from_invocation(record)
                    .unwrap_or_else(|| unreachable!("pending dispatches are projection-safe"))
            })
            .collect()
    }

    /// Returns the earliest live global frontier barrier.
    #[must_use]
    pub(crate) fn minimum_blocked_frontier(&self) -> Option<SimMoment> {
        self.pending_dispatches()
            .filter_map(ActionEvaluationInvocationRecord::blocked_at_frontier)
            .min()
    }

    pub(crate) fn install_dispatch(
        &mut self,
        record: ActionEvaluationInvocationRecord,
        waiting: &ActionOpportunity,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        if !matches!(
            (&record.payload, &record.state),
            (
                ActionEvaluationInvocationPayload::Dispatchable(_),
                ActionEvaluationInvocationState::DispatchPending
            )
        ) {
            return Err(ActionEvaluationInvocationLedgerError::Transition(
                ActionEvaluationInvocationError::StateMismatch,
            ));
        }
        self.install_initial(record, waiting)
    }

    pub(crate) fn install_artifact_rejection(
        &mut self,
        record: ActionEvaluationInvocationRecord,
        waiting: &ActionOpportunity,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        let exact_rejection = matches!(
            (&record.payload, &record.state),
            (
                ActionEvaluationInvocationPayload::ArtifactRejected { failure },
                ActionEvaluationInvocationState::FallbackPending {
                    cause: ActionEvaluationFallbackCause::ArtifactRejected(cause),
                    ..
                }
            ) if failure == cause
        );
        if !exact_rejection {
            return Err(ActionEvaluationInvocationLedgerError::Transition(
                ActionEvaluationInvocationError::StateMismatch,
            ));
        }
        self.install_initial(record, waiting)
    }

    fn install_initial(
        &mut self,
        record: ActionEvaluationInvocationRecord,
        waiting: &ActionOpportunity,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        let invocation = record.invocation();
        if self.entries.contains_key(&invocation) {
            return Err(ActionEvaluationInvocationLedgerError::DuplicateInvocation { invocation });
        }
        if waiting.id() != record.opportunity()
            || waiting.version() != record.waiting_version()
            || waiting.evaluation_generation() != record.evaluation_generation()
            || waiting.state() != ActionOpportunityState::WaitingForEvaluation(invocation)
        {
            return Err(ActionEvaluationInvocationLedgerError::OpportunityMismatch {
                invocation,
                opportunity: record.opportunity(),
            });
        }
        self.entries.insert(invocation, record);
        Ok(self
            .entries
            .get(&invocation)
            .unwrap_or_else(|| unreachable!("inserted invocation must remain indexed")))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the ledger checks one complete result-capture transition atomically"
    )]
    pub(crate) fn capture_result(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
        capture: ActionEvaluationCaptureId,
        capture_fingerprint: ActionEvaluationCaptureFingerprint,
        artifact: ActionEvaluationResultArtifact,
        effective: SimMoment,
        scheduler_key: SchedulerKey,
        control: DeferredActionControlV1,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.capture_result(
                expected_waiting_version,
                capture,
                capture_fingerprint,
                artifact,
                effective,
                scheduler_key,
                control,
            )
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the ledger checks one complete rejected result-capture transition atomically"
    )]
    pub(crate) fn capture_artifact_rejection(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
        capture_fingerprint: ActionEvaluationCaptureFingerprint,
        failure: ActionEvaluationArtifactFailure,
        effective: SimMoment,
        scheduler_key: SchedulerKey,
        control: DeferredActionControlV1,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.capture_artifact_rejection(
                expected_waiting_version,
                capture_fingerprint,
                failure,
                effective,
                scheduler_key,
                control,
            )
        })
    }

    pub(crate) fn begin_fallback(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
        cause: ActionEvaluationFallbackCause,
        scheduler_key: SchedulerKey,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.begin_fallback(expected_waiting_version, cause, scheduler_key)
        })
    }

    pub(crate) fn begin_managed_fallback(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
        cause: ActionEvaluationFallbackCause,
        scheduler_key: SchedulerKey,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.begin_managed_fallback(expected_waiting_version, cause, scheduler_key)
        })
    }

    pub(crate) fn finish_applied(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
        result: ActionEvaluationResultId,
        freshness: ActionEvaluationResultFreshness,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.finish_applied(expected_waiting_version, result, freshness)
        })
    }

    pub(crate) fn finish_reinvoked(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
        result: ActionEvaluationResultId,
        successor: ActionEvaluationInvocationId,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.finish_reinvoked(expected_waiting_version, result, successor)
        })
    }

    pub(crate) fn finish_fallback(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        expected_waiting_version: ActionOpportunityVersion,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        self.transition(invocation, |record| {
            record.finish_fallback(expected_waiting_version)
        })
    }

    /// Installs one already sealed invocation successor against its exact
    /// retained predecessor digest.
    pub(crate) fn install_transition_exact(
        &mut self,
        expected_before: ActionEvaluationInvocationDigest,
        after: ActionEvaluationInvocationRecord,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        let invocation = after.invocation();
        let current = self
            .entries
            .get(&invocation)
            .ok_or(ActionEvaluationInvocationLedgerError::UnknownInvocation { invocation })?;
        let actual = current.digest();
        if actual != expected_before {
            return Err(
                ActionEvaluationInvocationLedgerError::TransitionSourceMismatch {
                    invocation,
                    expected: expected_before,
                    actual,
                },
            );
        }
        self.entries.insert(invocation, after);
        Ok(self
            .entries
            .get(&invocation)
            .unwrap_or_else(|| unreachable!("exact invocation successor must remain indexed")))
    }

    #[must_use]
    #[cfg(test)]
    fn digest(&self) -> ActionEvaluationInvocationLedgerDigest {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_LEDGER_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_LEDGER_SCHEMA_VERSION);
        writer.write_u64(self.entries.len() as u64);
        for (invocation, record) in &self.entries {
            write_fixed(&mut writer, invocation.as_bytes());
            write_blob(&mut writer, record.canonical_bytes().as_bytes());
        }
        ActionEvaluationInvocationLedgerDigest(
            ContentDigest::of_canonical(&writer.finish()).into_bytes(),
        )
    }

    fn transition(
        &mut self,
        invocation: ActionEvaluationInvocationId,
        build: impl FnOnce(
            &ActionEvaluationInvocationRecord,
        )
            -> Result<ActionEvaluationInvocationRecord, ActionEvaluationInvocationError>,
    ) -> Result<&ActionEvaluationInvocationRecord, ActionEvaluationInvocationLedgerError> {
        let current = self
            .entries
            .get(&invocation)
            .ok_or(ActionEvaluationInvocationLedgerError::UnknownInvocation { invocation })?;
        let successor = build(current)?;
        self.entries.insert(invocation, successor);
        Ok(self
            .entries
            .get(&invocation)
            .unwrap_or_else(|| unreachable!("transitioned invocation must remain indexed")))
    }
}

/// Checked retained payload of one result-capture request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ActionEvaluationCapturePayload {
    Result {
        result: ActionEvaluationResultId,
        artifact: ActionEvaluationResultArtifact,
    },
    ArtifactRejected {
        result: ActionEvaluationResultId,
        failure: ActionEvaluationArtifactFailure,
    },
}

impl ActionEvaluationCapturePayload {
    pub(crate) const fn result(&self) -> ActionEvaluationResultId {
        match self {
            Self::Result { result, .. } | Self::ArtifactRejected { result, .. } => *result,
        }
    }

    #[cfg(test)]
    pub(crate) const fn failure(&self) -> Option<ActionEvaluationArtifactFailure> {
        match self {
            Self::Result { .. } => None,
            Self::ArtifactRejected { failure, .. } => Some(*failure),
        }
    }
}

/// Canonical capture request resolved against one retained invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActionEvaluationCaptureRequest {
    capture: ActionEvaluationCaptureId,
    invocation: ActionEvaluationInvocationId,
    request: ActionEvaluationRequestId,
    effective: SimMoment,
    admission_mode: DeferredActionAdmissionModeV1,
    result_schema: ActionEvaluationArtifactSchemaId,
    payload: ActionEvaluationCapturePayload,
    fingerprint: ActionEvaluationCaptureFingerprint,
}

/// Why raw result bytes could not form a new capture for one invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionEvaluationCaptureRequestError {
    InvocationMismatch {
        expected: ActionEvaluationInvocationId,
        actual: ActionEvaluationInvocationId,
    },
    InvocationNotDispatchable {
        invocation: ActionEvaluationInvocationId,
    },
    InvocationNotPending {
        invocation: ActionEvaluationInvocationId,
    },
    TimingMismatch {
        admission_mode: DeferredActionAdmissionModeV1,
        supplied: ActionEvaluationCaptureTiming,
    },
    MissingBlockingFrontier {
        invocation: ActionEvaluationInvocationId,
    },
    EffectiveMomentNotAfterCreation {
        effective: SimMoment,
        creation: SimMoment,
    },
    EffectiveMomentBeforeFrontier {
        effective: SimMoment,
        frontier: SimMoment,
    },
    ResultSchemaMismatch {
        expected: ActionEvaluationArtifactSchemaId,
        actual: ActionEvaluationArtifactSchemaId,
    },
    Artifact(ActionEvaluationArtifactError),
}

impl From<ActionEvaluationArtifactError> for ActionEvaluationCaptureRequestError {
    fn from(error: ActionEvaluationArtifactError) -> Self {
        Self::Artifact(error)
    }
}

impl ActionEvaluationCaptureRequest {
    /// Resolves stable body identity without consulting mutable admission state.
    ///
    /// Callers can therefore classify an exact retained retry before applying
    /// the current-state checks in [`Self::validate_new`].
    pub(crate) fn resolve(
        submission: ActionEvaluationResultSubmission,
        record: &ActionEvaluationInvocationRecord,
        control: DeferredActionControlV1,
    ) -> Result<Self, ActionEvaluationCaptureRequestError> {
        if submission.invocation != record.invocation() {
            return Err(ActionEvaluationCaptureRequestError::InvocationMismatch {
                expected: record.invocation(),
                actual: submission.invocation,
            });
        }
        let request = record.request_id().ok_or(
            ActionEvaluationCaptureRequestError::InvocationNotDispatchable {
                invocation: record.invocation(),
            },
        )?;
        let effective = match (record.admission_mode(), submission.timing) {
            (
                DeferredActionAdmissionModeV1::FrontierBlocking,
                ActionEvaluationCaptureTiming::InvocationFrontier,
            ) => record.blocked_at_frontier().ok_or(
                ActionEvaluationCaptureRequestError::MissingBlockingFrontier {
                    invocation: record.invocation(),
                },
            )?,
            (
                DeferredActionAdmissionModeV1::HostScheduled,
                ActionEvaluationCaptureTiming::HostScheduled(effective),
            ) => {
                if effective <= record.creation_moment() {
                    return Err(
                        ActionEvaluationCaptureRequestError::EffectiveMomentNotAfterCreation {
                            effective,
                            creation: record.creation_moment(),
                        },
                    );
                }
                effective
            }
            (admission_mode, supplied) => {
                return Err(ActionEvaluationCaptureRequestError::TimingMismatch {
                    admission_mode,
                    supplied,
                });
            }
        };

        let payload = match ActionEvaluationResultArtifact::new(
            submission.result_schema,
            submission.bytes.into_vec(),
            control,
        ) {
            Ok(artifact) => {
                let result = ActionEvaluationResultId::derive(request, &artifact);
                ActionEvaluationCapturePayload::Result { result, artifact }
            }
            Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => {
                let result = ActionEvaluationResultId::derive_parts(
                    request,
                    failure.schema(),
                    failure.digest(),
                );
                ActionEvaluationCapturePayload::ArtifactRejected { result, failure }
            }
            Err(error) => return Err(error.into()),
        };
        let fingerprint = match &payload {
            ActionEvaluationCapturePayload::Result { result, artifact } => {
                ActionEvaluationCaptureFingerprint::derive(
                    record.invocation(),
                    request,
                    *result,
                    effective,
                    record.admission_mode(),
                    artifact,
                )
            }
            ActionEvaluationCapturePayload::ArtifactRejected { result, failure } => {
                ActionEvaluationCaptureFingerprint::derive_failure(
                    record.invocation(),
                    request,
                    *result,
                    effective,
                    record.admission_mode(),
                    *failure,
                )
            }
        };
        Ok(Self {
            capture: submission.capture,
            invocation: record.invocation(),
            request,
            effective,
            admission_mode: record.admission_mode(),
            result_schema: submission.result_schema,
            payload,
            fingerprint,
        })
    }

    /// Checks state that exact retained replay deliberately bypasses.
    pub(crate) fn validate_new(
        &self,
        record: &ActionEvaluationInvocationRecord,
        admission_frontier: SimMoment,
    ) -> Result<(), ActionEvaluationCaptureRequestError> {
        if record.invocation() != self.invocation {
            return Err(ActionEvaluationCaptureRequestError::InvocationMismatch {
                expected: record.invocation(),
                actual: self.invocation,
            });
        }
        if !matches!(
            record.state(),
            ActionEvaluationInvocationState::DispatchPending
        ) {
            return Err(ActionEvaluationCaptureRequestError::InvocationNotPending {
                invocation: self.invocation,
            });
        }
        let expected_schema = record.result_schema().ok_or(
            ActionEvaluationCaptureRequestError::InvocationNotDispatchable {
                invocation: self.invocation,
            },
        )?;
        if self.result_schema != expected_schema {
            return Err(ActionEvaluationCaptureRequestError::ResultSchemaMismatch {
                expected: expected_schema,
                actual: self.result_schema,
            });
        }
        if matches!(
            self.admission_mode,
            DeferredActionAdmissionModeV1::HostScheduled
        ) && self.effective < admission_frontier
        {
            return Err(
                ActionEvaluationCaptureRequestError::EffectiveMomentBeforeFrontier {
                    effective: self.effective,
                    frontier: admission_frontier,
                },
            );
        }
        Ok(())
    }

    pub(crate) const fn capture(&self) -> ActionEvaluationCaptureId {
        self.capture
    }

    pub(crate) const fn invocation(&self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    pub(crate) const fn effective(&self) -> SimMoment {
        self.effective
    }

    #[cfg(test)]
    pub(crate) const fn admission_mode(&self) -> DeferredActionAdmissionModeV1 {
        self.admission_mode
    }

    pub(crate) const fn fingerprint(&self) -> ActionEvaluationCaptureFingerprint {
        self.fingerprint
    }

    pub(crate) const fn payload(&self) -> &ActionEvaluationCapturePayload {
        &self.payload
    }

    pub(crate) fn outcome(&self, record: AuthorityRecordId) -> ActionEvaluationCaptureOutcome {
        match &self.payload {
            ActionEvaluationCapturePayload::Result { result, .. } => {
                ActionEvaluationCaptureOutcome::ResultCaptured {
                    record,
                    invocation: self.invocation,
                    result: *result,
                    effective: self.effective,
                }
            }
            ActionEvaluationCapturePayload::ArtifactRejected { failure, .. } => {
                ActionEvaluationCaptureOutcome::ArtifactRejected {
                    record,
                    invocation: self.invocation,
                    failure: *failure,
                    effective: self.effective,
                }
            }
        }
    }

    pub(crate) fn write_canonical(&self, writer: &mut CanonicalWriter) {
        writer.write_u64(self.capture.get());
        write_fixed(writer, self.invocation.as_bytes());
        write_fixed(writer, self.request.as_bytes());
        write_fixed(writer, self.payload.result().as_bytes());
        write_moment(writer, self.effective);
        writer.write_discriminant(admission_mode_tag(self.admission_mode));
        write_fixed(writer, self.result_schema.as_bytes());
        match &self.payload {
            ActionEvaluationCapturePayload::Result { artifact, .. } => {
                writer.write_discriminant(0);
                artifact.write_canonical(writer);
            }
            ActionEvaluationCapturePayload::ArtifactRejected { failure, .. } => {
                writer.write_discriminant(1);
                failure.write_canonical(writer);
            }
        }
        write_fixed(writer, self.fingerprint.as_bytes());
    }
}

/// Classification of one action-evaluation capture identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionEvaluationCaptureLookup {
    Absent,
    RetainedExact(ActionEvaluationCaptureOutcome),
    IdReuseMismatch,
}

/// Retained exact-retry witness for one published result capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActionEvaluationCaptureLedgerEntry {
    invocation: ActionEvaluationInvocationId,
    fingerprint: ActionEvaluationCaptureFingerprint,
    outcome: ActionEvaluationCaptureOutcome,
}

impl ActionEvaluationCaptureLedgerEntry {
    pub(crate) const fn invocation(self) -> ActionEvaluationInvocationId {
        self.invocation
    }

    pub(crate) const fn fingerprint(self) -> ActionEvaluationCaptureFingerprint {
        self.fingerprint
    }

    pub(crate) const fn outcome(self) -> ActionEvaluationCaptureOutcome {
        self.outcome
    }
}

/// Insertion of a capture identity that already has a retained outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActionEvaluationCaptureLedgerError {
    NotAbsent { capture: ActionEvaluationCaptureId },
    OutcomeMismatch { capture: ActionEvaluationCaptureId },
}

/// Exact capture outcomes keyed in their own non-command namespace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ActionEvaluationCaptureLedger {
    entries: BTreeMap<ActionEvaluationCaptureId, ActionEvaluationCaptureLedgerEntry>,
}

impl ActionEvaluationCaptureLedger {
    #[must_use]
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub(crate) fn get(
        &self,
        capture: ActionEvaluationCaptureId,
    ) -> Option<ActionEvaluationCaptureLedgerEntry> {
        self.entries.get(&capture).copied()
    }

    #[must_use]
    pub(crate) fn classify(
        &self,
        capture: ActionEvaluationCaptureId,
        invocation: ActionEvaluationInvocationId,
        fingerprint: ActionEvaluationCaptureFingerprint,
    ) -> ActionEvaluationCaptureLookup {
        match self.get(capture) {
            None => ActionEvaluationCaptureLookup::Absent,
            Some(entry)
                if entry.invocation() == invocation && entry.fingerprint() == fingerprint =>
            {
                ActionEvaluationCaptureLookup::RetainedExact(entry.outcome())
            }
            Some(_) => ActionEvaluationCaptureLookup::IdReuseMismatch,
        }
    }

    pub(crate) fn insert_exact(
        &mut self,
        request: &ActionEvaluationCaptureRequest,
        outcome: ActionEvaluationCaptureOutcome,
    ) -> Result<(), ActionEvaluationCaptureLedgerError> {
        let matching_outcome = match (request.payload(), outcome) {
            (
                ActionEvaluationCapturePayload::Result { result, .. },
                ActionEvaluationCaptureOutcome::ResultCaptured {
                    invocation,
                    result: outcome_result,
                    effective,
                    ..
                },
            ) => {
                invocation == request.invocation
                    && outcome_result == *result
                    && effective == request.effective
            }
            (
                ActionEvaluationCapturePayload::ArtifactRejected { failure, .. },
                ActionEvaluationCaptureOutcome::ArtifactRejected {
                    invocation,
                    failure: outcome_failure,
                    effective,
                    ..
                },
            ) => {
                invocation == request.invocation
                    && outcome_failure == *failure
                    && effective == request.effective
            }
            _ => false,
        };
        if !matching_outcome {
            return Err(ActionEvaluationCaptureLedgerError::OutcomeMismatch {
                capture: request.capture,
            });
        }
        match self.entries.entry(request.capture) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(ActionEvaluationCaptureLedgerEntry {
                    invocation: request.invocation,
                    fingerprint: request.fingerprint,
                    outcome,
                });
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                Err(ActionEvaluationCaptureLedgerError::NotAbsent {
                    capture: request.capture,
                })
            }
        }
    }

    #[must_use]
    #[cfg(test)]
    fn digest(&self) -> ActionEvaluationCaptureLedgerDigest {
        let mut writer = CanonicalWriter::new(ACTION_EVALUATION_CAPTURE_LEDGER_DOMAIN);
        writer.write_u16(ACTION_EVALUATION_CAPTURE_LEDGER_SCHEMA_VERSION);
        writer.write_u64(self.entries.len() as u64);
        for (capture, entry) in &self.entries {
            writer.write_u64(capture.get());
            write_fixed(&mut writer, entry.invocation.as_bytes());
            write_fixed(&mut writer, entry.fingerprint.as_bytes());
            write_capture_outcome(&mut writer, entry.outcome);
        }
        ActionEvaluationCaptureLedgerDigest(
            ContentDigest::of_canonical(&writer.finish()).into_bytes(),
        )
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "one retained invocation is fenced by one complete actor-visible and timing coordinate"
)]
fn checked_invocation_control(
    invocation: ActionEvaluationInvocationId,
    opportunity: ActionOpportunityId,
    pre_wait_version: ActionOpportunityVersion,
    waiting_version: ActionOpportunityVersion,
    evaluation_generation: ActionEvaluationGeneration,
    policy_semantics: [u8; 32],
    action_input_fingerprint: [u8; 32],
    creation_moment: SimMoment,
    blocked_at_frontier: Option<SimMoment>,
    control: DeferredActionControlV1,
) -> Result<CheckedInvocationControl, ActionEvaluationInvocationError> {
    let expected_invocation = ActionEvaluationInvocationId::derive(
        opportunity,
        evaluation_generation,
        policy_semantics,
        action_input_fingerprint,
    );
    if invocation != expected_invocation {
        return Err(
            ActionEvaluationInvocationError::InvocationIdentityMismatch {
                expected: expected_invocation,
                actual: invocation,
            },
        );
    }
    if pre_wait_version.checked_next() != Some(waiting_version) {
        return Err(
            ActionEvaluationInvocationError::OpportunityVersionDiscontinuity {
                before: pre_wait_version,
                waiting: waiting_version,
            },
        );
    }
    let Some(admission_mode) = control.admission_mode() else {
        return Err(ActionEvaluationInvocationError::DeferredEvaluationDisabled);
    };
    let expected_blocking = matches!(
        (admission_mode, blocked_at_frontier),
        (DeferredActionAdmissionModeV1::FrontierBlocking, Some(_))
            | (DeferredActionAdmissionModeV1::HostScheduled, None)
    );
    if !expected_blocking {
        return Err(ActionEvaluationInvocationError::BlockingFrontierMismatch {
            admission_mode,
            blocked_at_frontier,
        });
    }
    if let Some(blocked_at_frontier) = blocked_at_frontier
        && blocked_at_frontier <= creation_moment
    {
        return Err(ActionEvaluationInvocationError::BlockingFrontierNotLater {
            creation: creation_moment,
            blocked_at_frontier,
        });
    }
    let Some(remaining_visible_reinvocations) = control.maximum_visible_reinvocations() else {
        return Err(ActionEvaluationInvocationError::DeferredEvaluationDisabled);
    };
    let Some(fallback) = control.fallback() else {
        return Err(ActionEvaluationInvocationError::DeferredEvaluationDisabled);
    };
    Ok(CheckedInvocationControl {
        admission_mode,
        remaining_visible_reinvocations,
        fallback,
    })
}

fn checked_artifact_length(
    failure: ActionEvaluationArtifactFailure,
    control: DeferredActionControlV1,
) -> Result<u32, ActionEvaluationArtifactError> {
    let maximum = maximum_artifact_bytes(failure.role(), control)?;
    let Ok(actual_u32) = u32::try_from(failure.actual_length()) else {
        return Err(ActionEvaluationArtifactError::LengthExceeded { maximum, failure });
    };
    if actual_u32 > maximum {
        Err(ActionEvaluationArtifactError::LengthExceeded { maximum, failure })
    } else {
        Ok(actual_u32)
    }
}

fn check_rejected_invocation_artifact(
    failure: ActionEvaluationArtifactFailure,
    expected_schema: Option<ActionEvaluationArtifactSchemaId>,
    control: DeferredActionControlV1,
) -> Result<(), ActionEvaluationInvocationError> {
    if matches!(failure.role(), ActionEvaluationArtifactRole::Result) {
        return Err(
            ActionEvaluationInvocationError::InvocationOpeningArtifactRoleMismatch {
                actual: failure.role(),
            },
        );
    }
    check_rejected_artifact(failure, failure.role(), expected_schema, control)
}

fn check_artifact_schema(
    expected: ActionEvaluationArtifactSchemaId,
    actual: ActionEvaluationArtifactSchemaId,
) -> Result<(), ActionEvaluationInvocationError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ActionEvaluationInvocationError::Artifact(
            ActionEvaluationArtifactError::SchemaMismatch { expected, actual },
        ))
    }
}

fn check_rejected_artifact(
    failure: ActionEvaluationArtifactFailure,
    expected_role: ActionEvaluationArtifactRole,
    expected_schema: Option<ActionEvaluationArtifactSchemaId>,
    control: DeferredActionControlV1,
) -> Result<(), ActionEvaluationInvocationError> {
    if failure.role() != expected_role {
        return Err(ActionEvaluationInvocationError::Artifact(
            ActionEvaluationArtifactError::RoleMismatch {
                expected: expected_role,
                actual: failure.role(),
            },
        ));
    }
    if let Some(expected) = expected_schema
        && failure.schema() != expected
    {
        return Err(ActionEvaluationInvocationError::Artifact(
            ActionEvaluationArtifactError::SchemaMismatch {
                expected,
                actual: failure.schema(),
            },
        ));
    }
    let maximum = maximum_artifact_bytes(failure.role(), control)
        .map_err(ActionEvaluationInvocationError::Artifact)?;
    if failure.actual_length() <= u64::from(maximum) {
        return Err(
            ActionEvaluationInvocationError::RejectedArtifactWithinBound { maximum, failure },
        );
    }
    Ok(())
}

fn maximum_artifact_bytes(
    role: ActionEvaluationArtifactRole,
    control: DeferredActionControlV1,
) -> Result<u32, ActionEvaluationArtifactError> {
    let maximum = match role {
        ActionEvaluationArtifactRole::Request => control.maximum_request_bytes(),
        ActionEvaluationArtifactRole::Result => control.maximum_result_bytes(),
        ActionEvaluationArtifactRole::PrivateContinuation => {
            control.maximum_private_continuation_bytes()
        }
        ActionEvaluationArtifactRole::PrivateReadWitness => control.maximum_private_witness_bytes(),
    };
    maximum
        .map(|value| value.get())
        .ok_or(ActionEvaluationArtifactError::DeferredEvaluationDisabled)
}

fn artifact_digest(
    role: ActionEvaluationArtifactRole,
    schema: ActionEvaluationArtifactSchemaId,
    bytes: &[u8],
) -> ActionEvaluationArtifactDigest {
    let mut writer = CanonicalWriter::new(ACTION_EVALUATION_ARTIFACT_DOMAIN);
    writer.write_u16(ACTION_EVALUATION_ARTIFACT_SCHEMA_VERSION);
    writer.write_discriminant(role.canonical_tag());
    write_fixed(&mut writer, schema.as_bytes());
    write_blob(&mut writer, bytes);
    ActionEvaluationArtifactDigest(ContentDigest::of_canonical(&writer.finish()).into_bytes())
}

fn write_invocation_cause(writer: &mut CanonicalWriter, cause: ActionEvaluationInvocationCause) {
    match cause {
        ActionEvaluationInvocationCause::Initial => writer.write_discriminant(0),
        ActionEvaluationInvocationCause::VisibleInputChanged { predecessor } => {
            writer.write_discriminant(1);
            write_fixed(writer, predecessor.as_bytes());
        }
    }
}

#[cfg(test)]
fn write_capture_outcome(writer: &mut CanonicalWriter, outcome: ActionEvaluationCaptureOutcome) {
    match outcome {
        ActionEvaluationCaptureOutcome::ResultCaptured {
            record,
            invocation,
            result,
            effective,
        } => {
            writer.write_discriminant(0);
            write_fixed(writer, record.as_bytes());
            write_fixed(writer, invocation.as_bytes());
            write_fixed(writer, result.as_bytes());
            write_moment(writer, effective);
        }
        ActionEvaluationCaptureOutcome::ArtifactRejected {
            record,
            invocation,
            failure,
            effective,
        } => {
            writer.write_discriminant(1);
            write_fixed(writer, record.as_bytes());
            write_fixed(writer, invocation.as_bytes());
            failure.write_canonical(writer);
            write_moment(writer, effective);
        }
    }
}

fn write_invocation_payload(
    writer: &mut CanonicalWriter,
    payload: &ActionEvaluationInvocationPayload,
) {
    match payload {
        ActionEvaluationInvocationPayload::Dispatchable(payload) => {
            writer.write_discriminant(0);
            payload.request.write_canonical(writer);
            write_fixed(writer, payload.request_id.as_bytes());
            write_fixed(writer, payload.result_schema.as_bytes());
            payload.private_continuation.write_canonical(writer);
            payload.private_read_witness.write_canonical(writer);
        }
        ActionEvaluationInvocationPayload::ArtifactRejected { failure } => {
            writer.write_discriminant(1);
            failure.write_canonical(writer);
        }
    }
}

fn write_invocation_state(writer: &mut CanonicalWriter, state: &ActionEvaluationInvocationState) {
    match state {
        ActionEvaluationInvocationState::DispatchPending => writer.write_discriminant(0),
        ActionEvaluationInvocationState::ResultCaptured {
            result,
            artifact,
            capture,
            capture_fingerprint,
            effective,
            scheduler_key,
        } => {
            writer.write_discriminant(1);
            write_fixed(writer, result.as_bytes());
            artifact.write_canonical(writer);
            writer.write_u64(capture.get());
            write_fixed(writer, capture_fingerprint.as_bytes());
            write_moment(writer, *effective);
            write_scheduler_key(writer, *scheduler_key);
        }
        ActionEvaluationInvocationState::FallbackPending {
            cause,
            scheduler_key,
        } => {
            writer.write_discriminant(2);
            write_fallback_cause(writer, *cause);
            write_scheduler_key(writer, *scheduler_key);
        }
        ActionEvaluationInvocationState::Terminal(terminal) => {
            writer.write_discriminant(3);
            write_terminal(writer, *terminal);
        }
    }
}

fn write_terminal(writer: &mut CanonicalWriter, terminal: ActionEvaluationTerminal) {
    match terminal {
        ActionEvaluationTerminal::Applied { result, freshness } => {
            writer.write_discriminant(0);
            write_fixed(writer, result.as_bytes());
            writer.write_discriminant(freshness_tag(freshness));
        }
        ActionEvaluationTerminal::Reinvoked { result, successor } => {
            writer.write_discriminant(1);
            write_fixed(writer, result.as_bytes());
            write_fixed(writer, successor.as_bytes());
        }
        ActionEvaluationTerminal::Failed { cause } => {
            writer.write_discriminant(2);
            write_fallback_cause(writer, cause);
        }
    }
}

fn write_scheduler_key(writer: &mut CanonicalWriter, key: SchedulerKey) {
    write_moment(writer, key.moment());
    writer.write_u32(key.lane().canonical_tag());
    writer.write_u64(key.sequence().get());
}

fn write_optional_moment(writer: &mut CanonicalWriter, moment: Option<SimMoment>) {
    match moment {
        None => writer.write_discriminant(0),
        Some(moment) => {
            writer.write_discriminant(1);
            write_moment(writer, moment);
        }
    }
}

const fn admission_mode_tag(mode: DeferredActionAdmissionModeV1) -> u32 {
    match mode {
        DeferredActionAdmissionModeV1::FrontierBlocking => 0,
        DeferredActionAdmissionModeV1::HostScheduled => 1,
    }
}

const fn fallback_tag(fallback: DeferredActionFallbackV1) -> u32 {
    match fallback {
        DeferredActionFallbackV1::FinishFailedOnLaterWake => 0,
    }
}

const fn freshness_tag(freshness: ActionEvaluationResultFreshness) -> u32 {
    match freshness {
        ActionEvaluationResultFreshness::Current => 0,
        ActionEvaluationResultFreshness::ProjectionRebound => 1,
        ActionEvaluationResultFreshness::ExecutionRevalidated => 2,
        ActionEvaluationResultFreshness::ProjectionReboundAndExecutionRevalidated => 3,
    }
}

const fn fallback_cause_tag(cause: ActionEvaluationFallbackCause) -> u32 {
    match cause {
        ActionEvaluationFallbackCause::Cancelled => 0,
        ActionEvaluationFallbackCause::TimedOut => 1,
        ActionEvaluationFallbackCause::HostFailure => 2,
        ActionEvaluationFallbackCause::InvalidResult => 3,
        ActionEvaluationFallbackCause::VisibleReinvocationExhausted => 4,
        ActionEvaluationFallbackCause::ArtifactRejected(_) => 5,
    }
}

fn write_fallback_cause(writer: &mut CanonicalWriter, cause: ActionEvaluationFallbackCause) {
    writer.write_discriminant(fallback_cause_tag(cause));
    if let ActionEvaluationFallbackCause::ArtifactRejected(failure) = cause {
        failure.write_canonical(writer);
    }
}

fn write_moment(writer: &mut CanonicalWriter, moment: SimMoment) {
    writer.write_u64(moment.time().ticks());
    writer.write_u64(moment.microstep().get());
}

fn write_fixed(writer: &mut CanonicalWriter, bytes: &[u8; 32]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("fixed-width action-evaluation identity must be canonical");
    }
}

fn write_blob(writer: &mut CanonicalWriter, bytes: &[u8]) {
    if writer.write_bytes(bytes).is_err() {
        unreachable!("allocated action-evaluation artifact must be canonical");
    }
}

#[cfg(test)]
mod tests {
    use world_core::{ActorId, EntityId, Microstep, SimTime};
    use world_model::{
        ActionInteractionScope, ActionOpportunityGeneration, ActionSponsor, ActorReactionCause,
        ContainmentInteractionScope,
    };

    use crate::authority::EpochIdentity;
    use crate::execution::{EpochLineageId, ExecutionSpecId, InitialStateRootId};
    use crate::scheduler::SchedulerSequence;

    use super::*;

    struct InvocationFixture {
        control: DeferredActionControlV1,
        waiting: ActionOpportunity,
        record: ActionEvaluationInvocationRecord,
    }

    fn moment(ticks: u64, microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(ticks), Microstep::new(microstep))
    }

    fn control(
        mode: DeferredActionAdmissionModeV1,
        reinvocations: u32,
        request: u32,
        result: u32,
        continuation: u32,
        witness: u32,
    ) -> DeferredActionControlV1 {
        DeferredActionControlV1::enabled(
            mode,
            reinvocations,
            request,
            result,
            continuation,
            witness,
        )
        .unwrap_or_else(|error| panic!("deferred-control fixture must be valid: {error}"))
    }

    fn opportunity(seed: u8) -> ActionOpportunity {
        let scope = ContainmentInteractionScope::new(
            EntityId::from_bytes([seed.wrapping_add(1); 32]),
            vec![EntityId::from_bytes([seed.wrapping_add(2); 32])],
            vec![EntityId::from_bytes([seed.wrapping_add(3); 32])],
            4,
        )
        .unwrap_or_else(|error| panic!("action-evaluation scope must be valid: {error}"));
        ActionOpportunity::open(
            ActorId::from_bytes([seed; 32]),
            ActionSponsor::actor_reaction(ActorReactionCause::from_bytes(
                [seed.wrapping_add(4); 32],
            )),
            ActionInteractionScope::containment(scope),
            ActionOpportunityGeneration::new(u64::from(seed)),
        )
    }

    fn cursor(seed: u8) -> AuthorityCursor {
        AuthorityCursor::root(
            EpochIdentity::new(
                EpochLineageId::from_bytes([seed; 32]),
                ExecutionSpecId::from_bytes([seed.wrapping_add(1); 32]),
            ),
            InitialStateRootId::from_bytes([seed.wrapping_add(2); 32]),
        )
    }

    fn invocation_fixture(
        seed: u8,
        mode: DeferredActionAdmissionModeV1,
        reinvocations: u32,
        blocked_at_frontier: Option<SimMoment>,
    ) -> InvocationFixture {
        let control = control(mode, reinvocations, 16, 16, 16, 16);
        let open = opportunity(seed);
        let pre_wait_version = open.version();
        let (waiting, invocation) = open
            .begin_evaluation(
                pre_wait_version,
                [seed.wrapping_add(5); 32],
                [seed.wrapping_add(6); 32],
            )
            .unwrap_or_else(|error| panic!("action evaluation must begin: {error}"));
        let request = ActionEvaluationRequestArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([seed.wrapping_add(7); 32]),
            vec![seed, seed.wrapping_add(1)],
            control,
        )
        .unwrap_or_else(|error| panic!("request artifact must be valid: {error}"));
        let continuation = ActionEvaluationPrivateContinuationArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([seed.wrapping_add(8); 32]),
            vec![seed.wrapping_add(2); 3],
            control,
        )
        .unwrap_or_else(|error| panic!("continuation artifact must be valid: {error}"));
        let witness = ActionEvaluationPrivateReadWitnessArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([seed.wrapping_add(9); 32]),
            vec![seed.wrapping_add(3); 4],
            control,
        )
        .unwrap_or_else(|error| panic!("witness artifact must be valid: {error}"));
        let record = ActionEvaluationInvocationRecord::dispatch_pending(
            invocation,
            waiting.id(),
            pre_wait_version,
            waiting.version(),
            waiting.evaluation_generation(),
            [seed.wrapping_add(5); 32],
            [seed.wrapping_add(6); 32],
            LifecycleImplementationId::from_bytes([seed.wrapping_add(10); 32]),
            request,
            ActionEvaluationArtifactSchemaId::from_bytes([seed.wrapping_add(11); 32]),
            continuation,
            witness,
            moment(4, 0),
            cursor(seed),
            blocked_at_frontier,
            control,
        )
        .unwrap_or_else(|error| panic!("dispatch record must be valid: {error:?}"));
        InvocationFixture {
            control,
            waiting,
            record,
        }
    }

    fn result_for(fixture: &InvocationFixture, byte: u8) -> ActionEvaluationResultArtifact {
        ActionEvaluationResultArtifact::new(
            fixture
                .record
                .result_schema()
                .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema")),
            vec![byte; 3],
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("result artifact must be valid: {error}"))
    }

    #[test]
    fn role_specific_artifacts_check_role_schema_length_bound_and_digest() {
        let control = control(DeferredActionAdmissionModeV1::HostScheduled, 1, 2, 3, 4, 5);
        let schema = ActionEvaluationArtifactSchemaId::from_bytes([0x11; 32]);
        let request = ActionEvaluationRequestArtifact::new(schema, vec![0x21; 2], control)
            .unwrap_or_else(|error| panic!("request at its bound must pass: {error}"));
        let result = ActionEvaluationResultArtifact::new(schema, vec![0x21; 3], control)
            .unwrap_or_else(|error| panic!("result at its bound must pass: {error}"));
        let continuation =
            ActionEvaluationPrivateContinuationArtifact::new(schema, vec![0x21; 4], control)
                .unwrap_or_else(|error| panic!("continuation at its bound must pass: {error}"));
        let witness =
            ActionEvaluationPrivateReadWitnessArtifact::new(schema, vec![0x21; 5], control)
                .unwrap_or_else(|error| panic!("witness at its bound must pass: {error}"));

        assert!(matches!(
            ActionEvaluationRequestArtifact::new(schema, vec![0; 3], control),
            Err(ActionEvaluationArtifactError::LengthExceeded {
                maximum: 2,
                failure,
            }) if failure.role() == ActionEvaluationArtifactRole::Request
                && failure.actual_length() == 3
                && failure.schema() == schema
        ));
        assert!(matches!(
            ActionEvaluationResultArtifact::new(schema, vec![0; 4], control),
            Err(ActionEvaluationArtifactError::LengthExceeded {
                maximum: 3,
                failure,
            }) if failure.role() == ActionEvaluationArtifactRole::Result
                && failure.actual_length() == 4
                && failure.schema() == schema
        ));
        assert!(matches!(
            ActionEvaluationPrivateContinuationArtifact::new(schema, vec![0; 5], control),
            Err(ActionEvaluationArtifactError::LengthExceeded {
                maximum: 4,
                failure,
            }) if failure.role() == ActionEvaluationArtifactRole::PrivateContinuation
                && failure.actual_length() == 5
                && failure.schema() == schema
        ));
        assert!(matches!(
            ActionEvaluationPrivateReadWitnessArtifact::new(schema, vec![0; 6], control),
            Err(ActionEvaluationArtifactError::LengthExceeded {
                maximum: 5,
                failure,
            }) if failure.role() == ActionEvaluationArtifactRole::PrivateReadWitness
                && failure.actual_length() == 6
                && failure.schema() == schema
        ));
        assert_ne!(request.digest(), result.digest());
        assert_ne!(result.digest(), continuation.digest());
        assert_ne!(continuation.digest(), witness.digest());

        assert!(matches!(
            ActionEvaluationRequestArtifact::from_recorded(
                ActionEvaluationArtifactRole::Result,
                schema,
                schema,
                request.length(),
                request.bytes().to_vec(),
                request.digest(),
                control,
            ),
            Err(ActionEvaluationArtifactError::RoleMismatch { .. })
        ));
        assert!(matches!(
            ActionEvaluationRequestArtifact::from_recorded(
                ActionEvaluationArtifactRole::Request,
                schema,
                ActionEvaluationArtifactSchemaId::from_bytes([0x12; 32]),
                request.length(),
                request.bytes().to_vec(),
                request.digest(),
                control,
            ),
            Err(ActionEvaluationArtifactError::SchemaMismatch { .. })
        ));
        assert!(matches!(
            ActionEvaluationRequestArtifact::from_recorded(
                ActionEvaluationArtifactRole::Request,
                schema,
                schema,
                request.length() + 1,
                request.bytes().to_vec(),
                request.digest(),
                control,
            ),
            Err(ActionEvaluationArtifactError::LengthMismatch {
                role: ActionEvaluationArtifactRole::Request,
                ..
            })
        ));
        assert!(matches!(
            ActionEvaluationRequestArtifact::from_recorded(
                ActionEvaluationArtifactRole::Request,
                schema,
                schema,
                request.length(),
                request.bytes().to_vec(),
                ActionEvaluationArtifactDigest::from_bytes([0xff; 32]),
                control,
            ),
            Err(ActionEvaluationArtifactError::DigestMismatch { .. })
        ));
        assert_eq!(
            ActionEvaluationRequestArtifact::new(
                schema,
                Vec::new(),
                DeferredActionControlV1::Disabled
            ),
            Err(ActionEvaluationArtifactError::DeferredEvaluationDisabled)
        );
    }

    #[test]
    fn request_result_and_capture_identities_bind_only_their_declared_preimages() {
        let control = control(
            DeferredActionAdmissionModeV1::HostScheduled,
            1,
            16,
            16,
            16,
            16,
        );
        let schema = ActionEvaluationArtifactSchemaId::from_bytes([0x31; 32]);
        let request = ActionEvaluationRequestArtifact::new(schema, vec![0x41], control)
            .unwrap_or_else(|error| panic!("request must be valid: {error}"));
        let changed_request = ActionEvaluationRequestArtifact::new(schema, vec![0x42], control)
            .unwrap_or_else(|error| panic!("changed request must be valid: {error}"));
        let invocation = ActionEvaluationInvocationId::from_bytes([0x51; 32]);
        let request_id = ActionEvaluationRequestId::derive(invocation, &request);
        assert_ne!(
            request_id,
            ActionEvaluationRequestId::derive(
                ActionEvaluationInvocationId::from_bytes([0x52; 32]),
                &request,
            )
        );
        assert_ne!(
            request_id,
            ActionEvaluationRequestId::derive(invocation, &changed_request)
        );

        let result = ActionEvaluationResultArtifact::new(schema, vec![0x61], control)
            .unwrap_or_else(|error| panic!("result must be valid: {error}"));
        let changed_result = ActionEvaluationResultArtifact::new(schema, vec![0x62], control)
            .unwrap_or_else(|error| panic!("changed result must be valid: {error}"));
        let result_id = ActionEvaluationResultId::derive(request_id, &result);
        assert_ne!(
            result_id,
            ActionEvaluationResultId::derive(
                ActionEvaluationRequestId::from_bytes([0x53; 32]),
                &result,
            )
        );
        assert_ne!(
            result_id,
            ActionEvaluationResultId::derive(request_id, &changed_result)
        );

        let effective = moment(8, 1);
        let fingerprint = ActionEvaluationCaptureFingerprint::derive(
            invocation,
            request_id,
            result_id,
            effective,
            DeferredActionAdmissionModeV1::HostScheduled,
            &result,
        );
        assert_eq!(ActionEvaluationCaptureId::new(7).get(), 7);
        assert_eq!(
            fingerprint,
            ActionEvaluationCaptureFingerprint::derive(
                invocation,
                request_id,
                result_id,
                effective,
                DeferredActionAdmissionModeV1::HostScheduled,
                &result,
            )
        );
        assert_ne!(
            fingerprint,
            ActionEvaluationCaptureFingerprint::derive(
                invocation,
                request_id,
                result_id,
                moment(8, 2),
                DeferredActionAdmissionModeV1::HostScheduled,
                &result,
            )
        );
        assert_eq!(
            [
                ActionEvaluationResultFreshness::Current,
                ActionEvaluationResultFreshness::ProjectionRebound,
                ActionEvaluationResultFreshness::ExecutionRevalidated,
                ActionEvaluationResultFreshness::ProjectionReboundAndExecutionRevalidated,
            ]
            .map(freshness_tag),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn invocation_construction_recomputes_identity_and_requires_a_later_blocker() {
        let fixture =
            invocation_fixture(0x69, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let request = fixture
            .record
            .request()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a request"))
            .clone();
        let result_schema = fixture
            .record
            .result_schema()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema"));
        let continuation = fixture
            .record
            .private_continuation()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a continuation"))
            .clone();
        let witness = fixture
            .record
            .private_read_witness()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a witness"))
            .clone();
        assert!(matches!(
            ActionEvaluationInvocationRecord::dispatch_pending(
                ActionEvaluationInvocationId::from_bytes([0xff; 32]),
                fixture.record.opportunity(),
                fixture.record.pre_wait_version(),
                fixture.record.waiting_version(),
                fixture.record.evaluation_generation(),
                *fixture.record.policy_semantics(),
                *fixture.record.action_input_fingerprint(),
                fixture.record.implementation(),
                request.clone(),
                result_schema,
                continuation.clone(),
                witness.clone(),
                fixture.record.creation_moment(),
                fixture.record.source_cursor(),
                None,
                fixture.control,
            ),
            Err(ActionEvaluationInvocationError::InvocationIdentityMismatch { .. })
        ));

        let frontier_control = control(
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            16,
            16,
            16,
            16,
        );
        assert!(matches!(
            ActionEvaluationInvocationRecord::dispatch_pending(
                fixture.record.invocation(),
                fixture.record.opportunity(),
                fixture.record.pre_wait_version(),
                fixture.record.waiting_version(),
                fixture.record.evaluation_generation(),
                *fixture.record.policy_semantics(),
                *fixture.record.action_input_fingerprint(),
                fixture.record.implementation(),
                request,
                result_schema,
                continuation,
                witness,
                fixture.record.creation_moment(),
                fixture.record.source_cursor(),
                Some(fixture.record.creation_moment()),
                frontier_control,
            ),
            Err(ActionEvaluationInvocationError::BlockingFrontierNotLater { .. })
        ));
    }

    #[test]
    fn invocation_ledger_fences_capture_completion_and_late_transitions() {
        let fixture =
            invocation_fixture(0x71, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let invocation = fixture.record.invocation();
        let waiting_version = fixture.waiting.version();
        let result = result_for(&fixture, 0x72);
        let request_id = fixture
            .record
            .request_id()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a request identity"));
        let result_id = ActionEvaluationResultId::derive(request_id, &result);
        let effective = moment(6, 1);
        let key = SchedulerKey::new(
            effective,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(3),
        );
        let fingerprint = ActionEvaluationCaptureFingerprint::derive(
            invocation,
            request_id,
            result_id,
            effective,
            fixture.record.admission_mode(),
            &result,
        );
        let mut ledger = ActionEvaluationInvocationLedger::default();
        ledger
            .install_dispatch(fixture.record, &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch must install: {error:?}"));

        ledger
            .capture_result(
                invocation,
                waiting_version,
                ActionEvaluationCaptureId::new(9),
                fingerprint,
                result,
                effective,
                key,
                fixture.control,
            )
            .unwrap_or_else(|error| panic!("matching capture must install: {error:?}"));
        assert!(matches!(
            ledger.begin_fallback(
                invocation,
                waiting_version,
                ActionEvaluationFallbackCause::InvalidResult,
                key,
            ),
            Err(ActionEvaluationInvocationLedgerError::Transition(
                ActionEvaluationInvocationError::SchedulerBindingMismatch
            ))
        ));
        assert!(matches!(
            ledger.finish_applied(
                invocation,
                ActionOpportunityVersion::INITIAL,
                result_id,
                ActionEvaluationResultFreshness::Current,
            ),
            Err(ActionEvaluationInvocationLedgerError::Transition(
                ActionEvaluationInvocationError::StaleWaitingVersion { .. }
            ))
        ));
        let terminal = ledger
            .finish_applied(
                invocation,
                waiting_version,
                result_id,
                ActionEvaluationResultFreshness::ProjectionReboundAndExecutionRevalidated,
            )
            .unwrap_or_else(|error| panic!("captured result must complete: {error:?}"));
        assert_eq!(
            terminal.state(),
            &ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Applied {
                result: result_id,
                freshness:
                    ActionEvaluationResultFreshness::ProjectionReboundAndExecutionRevalidated,
            })
        );
        assert!(matches!(
            ledger.begin_managed_fallback(
                invocation,
                waiting_version,
                ActionEvaluationFallbackCause::TimedOut,
                key,
            ),
            Err(ActionEvaluationInvocationLedgerError::Transition(
                ActionEvaluationInvocationError::StateMismatch
            ))
        ));
    }

    #[test]
    fn artifact_rejection_starts_in_fallback_and_is_never_dispatchable() {
        let strict_control = control(DeferredActionAdmissionModeV1::HostScheduled, 1, 2, 8, 8, 8);
        let schema = ActionEvaluationArtifactSchemaId::from_bytes([0x83; 32]);
        let failure =
            match ActionEvaluationRequestArtifact::new(schema, vec![0x84; 3], strict_control) {
                Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => failure,
                other => panic!("oversize request must retain bounded failure evidence: {other:?}"),
            };
        assert_eq!(failure.role(), ActionEvaluationArtifactRole::Request);
        assert_eq!(failure.schema(), schema);
        assert_eq!(failure.actual_length(), 3);
        assert_ne!(
            failure.digest(),
            artifact_digest(ActionEvaluationArtifactRole::Request, schema, &[0x85; 3])
        );

        let open = opportunity(0x84);
        let policy_semantics = [0x85; 32];
        let action_input_fingerprint = [0x86; 32];
        let pre_wait_version = open.version();
        let (waiting, invocation) = open
            .begin_evaluation(pre_wait_version, policy_semantics, action_input_fingerprint)
            .unwrap_or_else(|error| panic!("rejected evaluation must still begin: {error}"));
        let due = moment(5, 0);
        let result_failure =
            match ActionEvaluationResultArtifact::new(schema, vec![0x85; 9], strict_control) {
                Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => failure,
                other => panic!("oversize result must retain bounded failure evidence: {other:?}"),
            };
        assert_eq!(
            ActionEvaluationInvocationRecord::artifact_rejected(
                invocation,
                waiting.id(),
                pre_wait_version,
                waiting.version(),
                waiting.evaluation_generation(),
                policy_semantics,
                action_input_fingerprint,
                LifecycleImplementationId::from_bytes([0x87; 32]),
                result_failure,
                moment(4, 0),
                cursor(0x84),
                None,
                SchedulerKey::new(
                    due,
                    SchedulerLaneV2::ActionEvaluation,
                    SchedulerSequence::new(7),
                ),
                strict_control,
            ),
            Err(
                ActionEvaluationInvocationError::InvocationOpeningArtifactRoleMismatch {
                    actual: ActionEvaluationArtifactRole::Result,
                }
            )
        );
        let relaxed_control = control(DeferredActionAdmissionModeV1::HostScheduled, 1, 4, 8, 8, 8);
        assert_eq!(
            ActionEvaluationInvocationRecord::artifact_rejected(
                invocation,
                waiting.id(),
                pre_wait_version,
                waiting.version(),
                waiting.evaluation_generation(),
                policy_semantics,
                action_input_fingerprint,
                LifecycleImplementationId::from_bytes([0x87; 32]),
                failure,
                moment(4, 0),
                cursor(0x84),
                None,
                SchedulerKey::new(
                    due,
                    SchedulerLaneV2::ActionEvaluation,
                    SchedulerSequence::new(7),
                ),
                relaxed_control,
            ),
            Err(
                ActionEvaluationInvocationError::RejectedArtifactWithinBound {
                    maximum: 4,
                    failure,
                }
            )
        );
        let record = ActionEvaluationInvocationRecord::artifact_rejected(
            invocation,
            waiting.id(),
            pre_wait_version,
            waiting.version(),
            waiting.evaluation_generation(),
            policy_semantics,
            action_input_fingerprint,
            LifecycleImplementationId::from_bytes([0x87; 32]),
            failure,
            moment(4, 0),
            cursor(0x84),
            None,
            SchedulerKey::new(
                due,
                SchedulerLaneV2::ActionEvaluation,
                SchedulerSequence::new(7),
            ),
            strict_control,
        )
        .unwrap_or_else(|error| panic!("artifact rejection must be retainable: {error:?}"));
        assert_eq!(
            record.payload(),
            &ActionEvaluationInvocationPayload::ArtifactRejected { failure }
        );
        assert!(matches!(
            record.state(),
            ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(actual),
                ..
            } if *actual == failure
        ));

        let mut ledger = ActionEvaluationInvocationLedger::default();
        ledger
            .install_artifact_rejection(record, &waiting)
            .unwrap_or_else(|error| panic!("artifact rejection must install: {error:?}"));
        assert_eq!(ledger.pending_dispatches().count(), 0);
        assert_eq!(
            ledger
                .finish_fallback(invocation, waiting.version())
                .unwrap_or_else(|error| panic!("artifact fallback must finish: {error:?}"))
                .state(),
            &ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Failed {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(failure),
            })
        );
    }

    #[test]
    fn visible_reinvocation_is_linked_advances_generation_and_decrements_budget() {
        let fixture =
            invocation_fixture(0x88, DeferredActionAdmissionModeV1::HostScheduled, 2, None);
        let predecessor_invocation = fixture.record.invocation();
        let predecessor_waiting = fixture.waiting.version();
        let result = result_for(&fixture, 0x89);
        let request_id = fixture
            .record
            .request_id()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a request identity"));
        let result_id = ActionEvaluationResultId::derive(request_id, &result);
        let effective = moment(6, 0);
        let capture_key = SchedulerKey::new(
            effective,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(8),
        );
        let fingerprint = ActionEvaluationCaptureFingerprint::derive(
            predecessor_invocation,
            request_id,
            result_id,
            effective,
            fixture.record.admission_mode(),
            &result,
        );
        let reopened = fixture
            .waiting
            .reopen_for_visible_reinvocation(predecessor_waiting, predecessor_invocation)
            .unwrap_or_else(|error| panic!("waiting opportunity must reopen: {error}"));
        let successor_policy = *fixture.record.policy_semantics();
        let changed_policy = [0x8a; 32];
        let successor_input = [0x8b; 32];
        let (successor_waiting, successor_invocation) = reopened
            .begin_evaluation(reopened.version(), successor_policy, successor_input)
            .unwrap_or_else(|error| panic!("reopened opportunity must begin successor: {error}"));

        let mut ledger = ActionEvaluationInvocationLedger::default();
        ledger
            .install_dispatch(fixture.record, &fixture.waiting)
            .unwrap_or_else(|error| panic!("predecessor must install: {error:?}"));
        ledger
            .capture_result(
                predecessor_invocation,
                predecessor_waiting,
                ActionEvaluationCaptureId::new(11),
                fingerprint,
                result,
                effective,
                capture_key,
                fixture.control,
            )
            .unwrap_or_else(|error| panic!("predecessor result must capture: {error:?}"));
        let captured_predecessor = ledger
            .get(predecessor_invocation)
            .unwrap_or_else(|| panic!("captured predecessor must remain retained"))
            .clone();
        let predecessor = ledger
            .finish_reinvoked(
                predecessor_invocation,
                predecessor_waiting,
                result_id,
                successor_invocation,
            )
            .unwrap_or_else(|error| panic!("predecessor must link successor: {error:?}"))
            .clone();

        let request_schema = predecessor
            .request()
            .unwrap_or_else(|| panic!("predecessor must retain request schema"))
            .schema();
        let continuation_schema = predecessor
            .private_continuation()
            .unwrap_or_else(|| panic!("predecessor must retain a continuation schema"))
            .schema();
        let witness_schema = predecessor
            .private_read_witness()
            .unwrap_or_else(|| panic!("predecessor must retain a witness schema"))
            .schema();
        let request =
            ActionEvaluationRequestArtifact::new(request_schema, vec![0x8d; 2], fixture.control)
                .unwrap_or_else(|error| panic!("successor request must be valid: {error}"));
        let continuation = ActionEvaluationPrivateContinuationArtifact::new(
            continuation_schema,
            vec![0x8f; 3],
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("successor continuation must be valid: {error}"));
        let witness = ActionEvaluationPrivateReadWitnessArtifact::new(
            witness_schema,
            vec![0x91; 4],
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("successor witness must be valid: {error}"));
        assert_eq!(
            ActionEvaluationInvocationRecord::visible_reinvocation_dispatch_pending(
                &predecessor,
                successor_invocation,
                reopened.version(),
                successor_waiting.version(),
                successor_waiting.evaluation_generation(),
                changed_policy,
                successor_input,
                request.clone(),
                continuation.clone(),
                witness.clone(),
                moment(7, 0),
                cursor(0x89),
                None,
                fixture.control,
            ),
            Err(ActionEvaluationInvocationError::ReinvocationControlMismatch)
        );
        let changed_request = ActionEvaluationRequestArtifact::new(
            ActionEvaluationArtifactSchemaId::from_bytes([0x8c; 32]),
            vec![0x8d; 2],
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("alternate request must be bounded: {error}"));
        assert!(matches!(
            ActionEvaluationInvocationRecord::visible_reinvocation_dispatch_pending(
                &predecessor,
                successor_invocation,
                reopened.version(),
                successor_waiting.version(),
                successor_waiting.evaluation_generation(),
                successor_policy,
                successor_input,
                changed_request,
                continuation.clone(),
                witness.clone(),
                moment(7, 0),
                cursor(0x89),
                None,
                fixture.control,
            ),
            Err(ActionEvaluationInvocationError::Artifact(
                ActionEvaluationArtifactError::SchemaMismatch { .. }
            ))
        ));
        assert!(matches!(
            ActionEvaluationInvocationRecord::visible_reinvocation_dispatch_pending(
                &predecessor,
                successor_invocation,
                reopened.version(),
                successor_waiting.version(),
                predecessor.evaluation_generation(),
                successor_policy,
                successor_input,
                request.clone(),
                continuation.clone(),
                witness.clone(),
                moment(7, 0),
                cursor(0x89),
                None,
                fixture.control,
            ),
            Err(ActionEvaluationInvocationError::EvaluationGenerationDiscontinuity { .. })
        ));
        let successor = ActionEvaluationInvocationRecord::visible_reinvocation_dispatch_pending(
            &predecessor,
            successor_invocation,
            reopened.version(),
            successor_waiting.version(),
            successor_waiting.evaluation_generation(),
            successor_policy,
            successor_input,
            request,
            continuation,
            witness,
            moment(7, 0),
            cursor(0x89),
            None,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("linked successor must be valid: {error:?}"));
        assert_eq!(successor.remaining_visible_reinvocations(), 1);
        assert_eq!(
            successor.cause(),
            ActionEvaluationInvocationCause::VisibleInputChanged {
                predecessor: predecessor_invocation,
            }
        );
        assert_eq!(successor.policy_semantics(), &successor_policy);
        assert_eq!(successor.action_input_fingerprint(), &successor_input);
        ledger
            .install_dispatch(successor, &successor_waiting)
            .unwrap_or_else(|error| panic!("linked successor must install: {error:?}"));

        let rejected_policy = *predecessor.policy_semantics();
        let rejected_input = [0x92; 32];
        let (rejected_waiting, rejected_invocation) = reopened
            .begin_evaluation(reopened.version(), rejected_policy, rejected_input)
            .unwrap_or_else(|error| panic!("reopened opportunity must derive rejection: {error}"));
        let rejected_predecessor = captured_predecessor
            .finish_reinvoked(predecessor_waiting, result_id, rejected_invocation)
            .unwrap_or_else(|error| panic!("captured predecessor must link rejection: {error:?}"));
        let failure = match ActionEvaluationRequestArtifact::new(
            request_schema,
            vec![0x93; 17],
            fixture.control,
        ) {
            Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => failure,
            other => panic!("oversized successor request must yield evidence: {other:?}"),
        };
        let fallback_key = SchedulerKey::new(
            moment(8, 0),
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(9),
        );
        let result_schema = predecessor
            .result_schema()
            .unwrap_or_else(|| panic!("predecessor must retain result schema"));
        let result_failure = match ActionEvaluationResultArtifact::new(
            result_schema,
            vec![0x95; 17],
            fixture.control,
        ) {
            Err(ActionEvaluationArtifactError::LengthExceeded { failure, .. }) => failure,
            other => panic!("oversized result must yield failure evidence: {other:?}"),
        };
        assert_eq!(
            ActionEvaluationInvocationRecord::visible_reinvocation_artifact_rejected(
                &rejected_predecessor,
                rejected_invocation,
                reopened.version(),
                rejected_waiting.version(),
                rejected_waiting.evaluation_generation(),
                rejected_policy,
                rejected_input,
                result_failure,
                moment(7, 0),
                cursor(0x8a),
                None,
                fallback_key,
                fixture.control,
            ),
            Err(
                ActionEvaluationInvocationError::InvocationOpeningArtifactRoleMismatch {
                    actual: ActionEvaluationArtifactRole::Result,
                }
            )
        );
        assert!(matches!(
            ActionEvaluationInvocationRecord::visible_reinvocation_artifact_rejected(
                &rejected_predecessor,
                rejected_invocation,
                reopened.version(),
                rejected_waiting.version(),
                rejected_waiting.evaluation_generation(),
                [0x94; 32],
                rejected_input,
                failure,
                moment(7, 0),
                cursor(0x8a),
                None,
                fallback_key,
                fixture.control,
            ),
            Err(ActionEvaluationInvocationError::ReinvocationControlMismatch)
        ));
        let rejected = ActionEvaluationInvocationRecord::visible_reinvocation_artifact_rejected(
            &rejected_predecessor,
            rejected_invocation,
            reopened.version(),
            rejected_waiting.version(),
            rejected_waiting.evaluation_generation(),
            rejected_policy,
            rejected_input,
            failure,
            moment(7, 0),
            cursor(0x8a),
            None,
            fallback_key,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("linked artifact rejection must be valid: {error:?}"));
        assert_eq!(rejected.remaining_visible_reinvocations(), 1);
        assert_eq!(
            rejected.cause(),
            ActionEvaluationInvocationCause::VisibleInputChanged {
                predecessor: predecessor_invocation,
            }
        );
        assert_eq!(
            rejected.payload(),
            &ActionEvaluationInvocationPayload::ArtifactRejected { failure }
        );
        assert_eq!(
            rejected.state(),
            &ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(failure),
                scheduler_key: fallback_key,
            }
        );
    }

    #[test]
    fn fallback_transition_is_later_work_and_terminalizes_one_concrete_cause() {
        let fixture =
            invocation_fixture(0x81, DeferredActionAdmissionModeV1::HostScheduled, 0, None);
        let invocation = fixture.record.invocation();
        let waiting_version = fixture.waiting.version();
        let due = moment(6, 2);
        let key = SchedulerKey::new(
            due,
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(4),
        );
        let mut ledger = ActionEvaluationInvocationLedger::default();
        ledger
            .install_dispatch(fixture.record, &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch must install: {error:?}"));
        ledger
            .begin_managed_fallback(
                invocation,
                waiting_version,
                ActionEvaluationFallbackCause::HostFailure,
                key,
            )
            .unwrap_or_else(|error| panic!("fallback must begin: {error:?}"));
        assert_eq!(
            ledger
                .finish_fallback(invocation, waiting_version)
                .unwrap_or_else(|error| panic!("fallback must finish: {error:?}"))
                .state(),
            &ActionEvaluationInvocationState::Terminal(ActionEvaluationTerminal::Failed {
                cause: ActionEvaluationFallbackCause::HostFailure,
            })
        );
    }

    #[test]
    fn sealed_invocation_transition_requires_the_exact_before_digest() {
        let fixture =
            invocation_fixture(0x82, DeferredActionAdmissionModeV1::HostScheduled, 0, None);
        let before = fixture.record.clone();
        let due = moment(6, 3);
        let after = before
            .begin_managed_fallback(
                before.waiting_version(),
                ActionEvaluationFallbackCause::TimedOut,
                SchedulerKey::new(
                    due,
                    SchedulerLaneV2::ActionEvaluation,
                    SchedulerSequence::new(5),
                ),
            )
            .unwrap_or_else(|error| panic!("fallback successor must build: {error:?}"));
        let expected = before.digest();
        let mut ledger = ActionEvaluationInvocationLedger::default();
        ledger
            .install_dispatch(before, &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch must install: {error:?}"));
        assert!(matches!(
            ledger.install_transition_exact(
                ActionEvaluationInvocationDigest::from_bytes([0xff; 32]),
                after.clone(),
            ),
            Err(ActionEvaluationInvocationLedgerError::TransitionSourceMismatch { .. })
        ));
        assert_eq!(
            ledger
                .install_transition_exact(expected, after.clone())
                .unwrap_or_else(|error| panic!("exact successor must install: {error:?}")),
            &after
        );
    }

    #[test]
    fn pending_dispatches_are_sorted_and_minimum_blocker_tracks_only_live_dispatch() {
        let high_block = moment(9, 0);
        let low_block = moment(7, 0);
        let high = invocation_fixture(
            0x91,
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            Some(high_block),
        );
        let low = invocation_fixture(
            0x92,
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            Some(low_block),
        );
        let host = invocation_fixture(0x93, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let high_invocation = high.record.invocation();
        let high_waiting = high.waiting.version();
        let low_invocation = low.record.invocation();
        let low_waiting = low.waiting.version();
        let host_invocation = host.record.invocation();
        let mut expected = vec![high_invocation, low_invocation, host_invocation];
        expected.sort_unstable();

        let mut ledger = ActionEvaluationInvocationLedger::default();
        ledger
            .install_dispatch(host.record, &host.waiting)
            .unwrap_or_else(|error| panic!("host dispatch must install: {error:?}"));
        ledger
            .install_dispatch(low.record, &low.waiting)
            .unwrap_or_else(|error| panic!("low blocker must install: {error:?}"));
        ledger
            .install_dispatch(high.record, &high.waiting)
            .unwrap_or_else(|error| panic!("high blocker must install: {error:?}"));
        assert_eq!(
            ledger
                .pending_dispatches()
                .map(ActionEvaluationInvocationRecord::invocation)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(ledger.minimum_blocked_frontier(), Some(low_block));

        let low_record = ledger
            .get(low_invocation)
            .unwrap_or_else(|| panic!("low invocation must remain retained"));
        let low_result = ActionEvaluationResultArtifact::new(
            low_record
                .result_schema()
                .unwrap_or_else(|| panic!("dispatch record must retain a result schema")),
            vec![0xa1; 3],
            low.control,
        )
        .unwrap_or_else(|error| panic!("low result must be valid: {error}"));
        let low_request = low_record
            .request_id()
            .unwrap_or_else(|| panic!("dispatch record must retain a request identity"));
        let low_result_id = ActionEvaluationResultId::derive(low_request, &low_result);
        let low_fingerprint = ActionEvaluationCaptureFingerprint::derive(
            low_invocation,
            low_request,
            low_result_id,
            low_block,
            low_record.admission_mode(),
            &low_result,
        );
        ledger
            .capture_result(
                low_invocation,
                low_waiting,
                ActionEvaluationCaptureId::new(10),
                low_fingerprint,
                low_result,
                low_block,
                SchedulerKey::new(
                    low_block,
                    SchedulerLaneV2::ActionEvaluation,
                    SchedulerSequence::new(5),
                ),
                low.control,
            )
            .unwrap_or_else(|error| panic!("low blocker capture must install: {error:?}"));
        assert_eq!(ledger.minimum_blocked_frontier(), Some(high_block));

        ledger
            .begin_managed_fallback(
                high_invocation,
                high_waiting,
                ActionEvaluationFallbackCause::Cancelled,
                SchedulerKey::new(
                    high_block,
                    SchedulerLaneV2::ActionEvaluation,
                    SchedulerSequence::new(6),
                ),
            )
            .unwrap_or_else(|error| panic!("high blocker fallback must install: {error:?}"));
        assert_eq!(ledger.minimum_blocked_frontier(), None);
        assert_eq!(
            ledger
                .pending_dispatches()
                .map(ActionEvaluationInvocationRecord::invocation)
                .collect::<Vec<_>>(),
            vec![host_invocation]
        );
    }

    #[test]
    fn capture_request_resolves_time_and_replays_in_its_own_ledger() {
        let fixture =
            invocation_fixture(0xa1, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let invocation = fixture.record.invocation();
        let schema = fixture
            .record
            .result_schema()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema"));
        let effective = moment(6, 0);
        let request = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::host_scheduled(
                ActionEvaluationCaptureId::new(17),
                invocation,
                effective,
                schema,
                vec![0xa2; 3],
            ),
            &fixture.record,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("capture body must resolve: {error:?}"));
        request
            .validate_new(&fixture.record, moment(5, 0))
            .unwrap_or_else(|error| panic!("future capture must be admissible: {error:?}"));
        assert_eq!(
            request.validate_new(&fixture.record, moment(7, 0)),
            Err(
                ActionEvaluationCaptureRequestError::EffectiveMomentBeforeFrontier {
                    effective,
                    frontier: moment(7, 0),
                }
            )
        );

        let record = AuthorityRecordId::from_bytes([0xa3; 32]);
        let outcome = request.outcome(record);
        let mut captures = ActionEvaluationCaptureLedger::default();
        assert!(captures.is_empty());
        captures
            .insert_exact(&request, outcome)
            .unwrap_or_else(|error| panic!("new capture must install: {error:?}"));
        assert_eq!(
            captures.classify(
                request.capture(),
                request.invocation(),
                request.fingerprint()
            ),
            ActionEvaluationCaptureLookup::RetainedExact(outcome)
        );
        assert_eq!(
            captures
                .get(request.capture())
                .unwrap_or_else(|| panic!("capture entry must remain retained"))
                .invocation(),
            invocation
        );

        let changed = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::host_scheduled(
                request.capture(),
                invocation,
                effective,
                schema,
                vec![0xa4; 3],
            ),
            &fixture.record,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("changed capture body must resolve: {error:?}"));
        assert_eq!(
            captures.classify(
                changed.capture(),
                changed.invocation(),
                changed.fingerprint()
            ),
            ActionEvaluationCaptureLookup::IdReuseMismatch
        );
        assert_eq!(
            captures.classify(
                request.capture(),
                ActionEvaluationInvocationId::from_bytes([0xff; 32]),
                request.fingerprint(),
            ),
            ActionEvaluationCaptureLookup::IdReuseMismatch
        );
        assert_eq!(
            captures.insert_exact(&request, outcome),
            Err(ActionEvaluationCaptureLedgerError::NotAbsent {
                capture: request.capture(),
            })
        );

        let second = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::host_scheduled(
                ActionEvaluationCaptureId::new(20),
                invocation,
                moment(6, 1),
                schema,
                vec![0xa5; 3],
            ),
            &fixture.record,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("second capture body must resolve: {error:?}"));
        let second_outcome = second.outcome(AuthorityRecordId::from_bytes([0xa6; 32]));
        captures
            .insert_exact(&second, second_outcome)
            .unwrap_or_else(|error| panic!("second capture must install: {error:?}"));
        let mut reverse = ActionEvaluationCaptureLedger::default();
        reverse
            .insert_exact(&second, second_outcome)
            .unwrap_or_else(|error| panic!("second capture must install first: {error:?}"));
        reverse
            .insert_exact(&request, outcome)
            .unwrap_or_else(|error| panic!("first capture must install second: {error:?}"));
        assert_eq!(captures.digest(), reverse.digest());
    }

    #[test]
    fn capture_time_is_fixed_by_mode_and_pending_projection_is_actor_safe() {
        let blocked = moment(5, 0);
        let fixture = invocation_fixture(
            0xa5,
            DeferredActionAdmissionModeV1::FrontierBlocking,
            1,
            Some(blocked),
        );
        let invocation = fixture.record.invocation();
        let schema = fixture
            .record
            .result_schema()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema"));
        let request = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::at_invocation_frontier(
                ActionEvaluationCaptureId::new(18),
                invocation,
                schema,
                vec![0xa6; 2],
            ),
            &fixture.record,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("blocking capture must resolve: {error:?}"));
        assert_eq!(request.effective(), blocked);
        assert_eq!(
            request.admission_mode(),
            DeferredActionAdmissionModeV1::FrontierBlocking
        );
        assert!(matches!(
            ActionEvaluationCaptureRequest::resolve(
                ActionEvaluationResultSubmission::host_scheduled(
                    ActionEvaluationCaptureId::new(18),
                    invocation,
                    moment(6, 0),
                    schema,
                    vec![0xa6; 2],
                ),
                &fixture.record,
                fixture.control,
            ),
            Err(ActionEvaluationCaptureRequestError::TimingMismatch { .. })
        ));

        let mut invocations = ActionEvaluationInvocationLedger::default();
        invocations
            .install_dispatch(fixture.record.clone(), &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch must install: {error:?}"));
        let pending = invocations.pending_raw();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].invocation(), invocation);
        assert_eq!(
            pending[0].request(),
            fixture
                .record
                .request_id()
                .unwrap_or_else(|| panic!("dispatch must retain request identity"))
        );
        assert_eq!(pending[0].implementation(), fixture.record.implementation());
        assert_eq!(
            pending[0].request_artifact(),
            fixture
                .record
                .request()
                .unwrap_or_else(|| panic!("dispatch must retain actor-safe request"))
        );
        assert_eq!(pending[0].result_schema(), schema);
        assert_eq!(
            pending[0].admission_mode(),
            DeferredActionAdmissionModeV1::FrontierBlocking
        );
    }

    #[test]
    fn oversized_capture_retains_only_failure_evidence_and_a_stable_replay_outcome() {
        let fixture =
            invocation_fixture(0xa7, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let schema = fixture
            .record
            .result_schema()
            .unwrap_or_else(|| panic!("dispatch fixture must retain a result schema"));
        let request = ActionEvaluationCaptureRequest::resolve(
            ActionEvaluationResultSubmission::host_scheduled(
                ActionEvaluationCaptureId::new(19),
                fixture.record.invocation(),
                moment(6, 0),
                schema,
                vec![0xa8; 17],
            ),
            &fixture.record,
            fixture.control,
        )
        .unwrap_or_else(|error| panic!("oversized bytes must reduce to evidence: {error:?}"));
        let failure = request
            .payload()
            .failure()
            .unwrap_or_else(|| panic!("oversized capture must retain failure evidence"));
        request
            .validate_new(&fixture.record, moment(5, 0))
            .unwrap_or_else(|error| {
                panic!("oversized capture timing must be admissible: {error:?}")
            });
        assert_eq!(failure.role(), ActionEvaluationArtifactRole::Result);
        assert_eq!(failure.schema(), schema);
        assert_eq!(failure.actual_length(), 17);
        assert!(matches!(
            request.outcome(AuthorityRecordId::from_bytes([0xa9; 32])),
            ActionEvaluationCaptureOutcome::ArtifactRejected {
                invocation,
                failure: actual,
                effective,
                ..
            } if invocation == fixture.record.invocation()
                && actual == failure
                && effective == moment(6, 0)
        ));

        let key = SchedulerKey::new(
            request.effective(),
            SchedulerLaneV2::ActionEvaluation,
            SchedulerSequence::new(4),
        );
        let mut invocations = ActionEvaluationInvocationLedger::default();
        invocations
            .install_dispatch(fixture.record, &fixture.waiting)
            .unwrap_or_else(|error| panic!("dispatch must install: {error:?}"));
        assert_eq!(
            invocations.begin_fallback(
                request.invocation(),
                fixture.waiting.version(),
                ActionEvaluationFallbackCause::ArtifactRejected(failure),
                key,
            ),
            Err(ActionEvaluationInvocationLedgerError::Transition(
                ActionEvaluationInvocationError::ArtifactRejectionRequiresCapture,
            ))
        );
        let transitioned = invocations
            .capture_artifact_rejection(
                request.invocation(),
                fixture.waiting.version(),
                request.fingerprint(),
                failure,
                request.effective(),
                key,
                fixture.control,
            )
            .unwrap_or_else(|error| panic!("rejected capture must transition: {error:?}"));
        assert_eq!(
            transitioned.state(),
            &ActionEvaluationInvocationState::FallbackPending {
                cause: ActionEvaluationFallbackCause::ArtifactRejected(failure),
                scheduler_key: key,
            }
        );
    }

    #[test]
    fn ledger_digest_is_invariant_to_installation_order() {
        let first = invocation_fixture(0xb1, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let second =
            invocation_fixture(0xb2, DeferredActionAdmissionModeV1::HostScheduled, 1, None);
        let mut forward = ActionEvaluationInvocationLedger::default();
        forward
            .install_dispatch(first.record.clone(), &first.waiting)
            .unwrap_or_else(|error| panic!("first dispatch must install: {error:?}"));
        forward
            .install_dispatch(second.record.clone(), &second.waiting)
            .unwrap_or_else(|error| panic!("second dispatch must install: {error:?}"));

        let mut reverse = ActionEvaluationInvocationLedger::default();
        reverse
            .install_dispatch(second.record, &second.waiting)
            .unwrap_or_else(|error| panic!("second dispatch must install: {error:?}"));
        reverse
            .install_dispatch(first.record, &first.waiting)
            .unwrap_or_else(|error| panic!("first dispatch must install: {error:?}"));
        assert_eq!(forward.digest(), reverse.digest());
    }
}
