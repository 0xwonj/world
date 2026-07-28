use core::fmt;

macro_rules! opaque_identity {
    ($(#[$metadata:meta])* $name:ident) => {
        $(#[$metadata])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub(crate) [u8; 32]);

        impl $name {
            /// Constructs an identity decoded from its owning protocol.
            ///
            /// This validates representation shape only. The enclosing
            /// protocol must still prove that the bytes belong to its exact
            /// payload.
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

opaque_identity!(
    /// Opportunity-local reference to an object the actor may use in policy
    /// input.
    ///
    /// It is deliberately not an alias for the exact model entity identity.
    ActorSafeObjectRef
);

opaque_identity!(
    /// Stable identity of one fully grounded action candidate.
    GroundedActionCandidateId
);

opaque_identity!(
    /// Canonical fingerprint of one bounded grounded-candidate set.
    GroundedCandidateSetFingerprint
);

opaque_identity!(
    /// Canonical fingerprint of the complete action-policy input.
    ActionInputFingerprint
);

opaque_identity!(
    /// Behavior identity of the grounder used to construct candidates.
    GroundingSemanticsId
);

opaque_identity!(
    /// Behavior identity of the action policy for which an input was built.
    ActionPolicySemanticsId
);

opaque_identity!(
    /// Schema identity of the canonical actor-safe action-context artifact.
    ActionContextPayloadSchemaId
);

opaque_identity!(
    /// Schema identity of the canonical private candidate-resolution artifact.
    CandidateResolutionTableSchemaId
);

opaque_identity!(
    /// Schema identity of the canonical action-projection witness artifact.
    ActionProjectionWitnessSchemaId
);

opaque_identity!(
    /// Schema identity of the canonical private execution-validation witness.
    ActionExecutionWitnessSchemaId
);

opaque_identity!(
    /// Schema identity of the canonical combined action-read witness.
    ActionReadWitnessSchemaId
);

opaque_identity!(
    /// Behavior identity of the evidence assimilator for which an input was
    /// built.
    EvidenceAssimilationSemanticsId
);

opaque_identity!(
    /// Canonical fingerprint of one evidence-assimilation input.
    EvidenceAssimilationInputFingerprint
);

opaque_identity!(
    /// Behavior identity of the appraisal evaluator for which an input was
    /// built.
    AppraisalEvaluatorSemanticsId
);

opaque_identity!(
    /// Canonical fingerprint of one containment-appraisal input.
    ContainmentAppraisalInputFingerprint
);

opaque_identity!(
    /// Behavior identity of the grounder used to construct intent candidates.
    IntentGroundingSemanticsId
);

opaque_identity!(
    /// Stable identity of one fully grounded intent candidate.
    GroundedIntentCandidateId
);

opaque_identity!(
    /// Canonical fingerprint of one grounded-intent candidate set.
    GroundedIntentCandidateSetFingerprint
);

opaque_identity!(
    /// Behavior identity of the intent policy for which an input was built.
    IntentPolicySemanticsId
);

opaque_identity!(
    /// Canonical fingerprint of one containment-intent policy input.
    IntentInputFingerprint
);

opaque_identity!(
    /// Behavior identity of the activity controller for which an input was
    /// built.
    ActivityControllerSemanticsId
);

opaque_identity!(
    /// Canonical fingerprint of one activity-initialization input.
    ActivityInitializationInputFingerprint
);

opaque_identity!(
    /// Canonical fingerprint of one activity-advancement input.
    ActivityAdvancementInputFingerprint
);
