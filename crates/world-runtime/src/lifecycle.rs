use world_core::{ActorId, SimMoment};
use world_model::{
    ActionOpportunityId, ContainmentAppraisalFingerprint, EvidenceDeliveryId, EvidenceRecord,
    IntentId, IntentVersion,
};

/// One actor-addressed direct observation proposed from a committed event.
///
/// The observation deliberately carries no evidence generation. Runtime
/// assigns that actor-local coordinate only while sealing the complete
/// post-commit batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceObservation {
    observer: ActorId,
    event_index: u32,
}

impl EvidenceObservation {
    /// Addresses one event in the paired retained dispatch to an observer.
    #[must_use]
    pub const fn direct(observer: ActorId, event_index: u32) -> Self {
        Self {
            observer,
            event_index,
        }
    }

    /// Returns the actor whose epistemic history will own the evidence.
    #[must_use]
    pub const fn observer(self) -> ActorId {
        self.observer
    }

    /// Returns the zero-based event coordinate in the retained dispatch.
    #[must_use]
    pub const fn event_index(self) -> u32 {
        self.event_index
    }
}

/// Generation of actor-local lifecycle work for one concrete role.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LifecycleGeneration(u64);

impl LifecycleGeneration {
    #[cfg(test)]
    pub(crate) const INITIAL: Self = Self(0);

    #[must_use]
    #[cfg(test)]
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub(crate) const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub(crate) const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Actor-local lifecycle computations whose redundant wake requests coalesce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LifecycleRole {
    Appraisal,
    IntentReview,
    ActivityInitialization,
    ActivityAdvance,
}

impl LifecycleRole {
    #[must_use]
    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::Appraisal => 0,
            Self::IntentReview => 1,
            Self::ActivityInitialization => 2,
            Self::ActivityAdvance => 3,
        }
    }

    #[must_use]
    pub(crate) const fn accepts(self, cause: LifecycleCause) -> bool {
        matches!(
            (self, cause),
            (Self::Appraisal, LifecycleCause::Evidence(_))
                | (Self::IntentReview, LifecycleCause::Appraisal { .. })
                | (Self::ActivityInitialization, LifecycleCause::Intent { .. })
                | (Self::ActivityAdvance, LifecycleCause::AttemptResolved(_))
        )
    }
}

/// Concrete semantic cause retained by lifecycle coalescing control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum LifecycleCause {
    Evidence(EvidenceDeliveryId),
    Appraisal {
        generation: LifecycleGeneration,
        material: ContainmentAppraisalFingerprint,
    },
    Intent {
        intent: IntentId,
        version: IntentVersion,
    },
    AttemptResolved(ActionOpportunityId),
}

/// One actor-addressed observation awaiting delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EvidenceDeliveryWork {
    evidence: EvidenceRecord,
    due: SimMoment,
}

impl EvidenceDeliveryWork {
    #[must_use]
    pub(crate) const fn new(evidence: EvidenceRecord, due: SimMoment) -> Self {
        Self { evidence, due }
    }

    #[must_use]
    pub(crate) const fn evidence(self) -> EvidenceRecord {
        self.evidence
    }

    #[must_use]
    pub(crate) const fn due(self) -> SimMoment {
        self.due
    }
}

/// One coalesced actor-relative appraisal generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppraisalWork {
    actor: ActorId,
    generation: LifecycleGeneration,
    due: SimMoment,
}

impl AppraisalWork {
    #[must_use]
    pub(crate) const fn new(
        actor: ActorId,
        generation: LifecycleGeneration,
        due: SimMoment,
    ) -> Self {
        Self {
            actor,
            generation,
            due,
        }
    }

    #[must_use]
    pub(crate) const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub(crate) const fn generation(self) -> LifecycleGeneration {
        self.generation
    }

    #[must_use]
    pub(crate) const fn due(self) -> SimMoment {
        self.due
    }
}

/// One coalesced actor-relative intent-review generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IntentReviewWork {
    actor: ActorId,
    generation: LifecycleGeneration,
    due: SimMoment,
}

impl IntentReviewWork {
    #[must_use]
    pub(crate) const fn new(
        actor: ActorId,
        generation: LifecycleGeneration,
        due: SimMoment,
    ) -> Self {
        Self {
            actor,
            generation,
            due,
        }
    }

    #[must_use]
    pub(crate) const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub(crate) const fn generation(self) -> LifecycleGeneration {
        self.generation
    }

    #[must_use]
    pub(crate) const fn due(self) -> SimMoment {
        self.due
    }
}

/// One coalesced actor-relative activity-initialization generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActivityInitializationWork {
    actor: ActorId,
    generation: LifecycleGeneration,
    due: SimMoment,
}

impl ActivityInitializationWork {
    #[must_use]
    pub(crate) const fn new(
        actor: ActorId,
        generation: LifecycleGeneration,
        due: SimMoment,
    ) -> Self {
        Self {
            actor,
            generation,
            due,
        }
    }

    #[must_use]
    pub(crate) const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub(crate) const fn generation(self) -> LifecycleGeneration {
        self.generation
    }

    #[must_use]
    pub(crate) const fn due(self) -> SimMoment {
        self.due
    }
}

/// Outcome-neutral continuation after one attempted action opportunity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptResolved {
    opportunity: ActionOpportunityId,
    due: SimMoment,
}

impl AttemptResolved {
    pub(crate) const fn new(opportunity: ActionOpportunityId, due: SimMoment) -> Self {
        Self { opportunity, due }
    }

    /// Returns the attempted opportunity without revealing its outcome.
    #[must_use]
    pub const fn opportunity(self) -> ActionOpportunityId {
        self.opportunity
    }

    /// Returns the exact delivery moment.
    #[must_use]
    pub const fn due(self) -> SimMoment {
        self.due
    }
}

/// One coalesced actor-relative activity-advance generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ActivityAdvanceWork {
    actor: ActorId,
    generation: LifecycleGeneration,
    due: SimMoment,
}

impl ActivityAdvanceWork {
    #[must_use]
    pub(crate) const fn new(
        actor: ActorId,
        generation: LifecycleGeneration,
        due: SimMoment,
    ) -> Self {
        Self {
            actor,
            generation,
            due,
        }
    }

    #[must_use]
    pub(crate) const fn actor(self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub(crate) const fn generation(self) -> LifecycleGeneration {
        self.generation
    }

    #[must_use]
    pub(crate) const fn due(self) -> SimMoment {
        self.due
    }
}

/// Closed family of deterministic lifecycle scheduler inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleWork {
    EvidenceDelivery(EvidenceDeliveryWork),
    Appraisal(AppraisalWork),
    IntentReview(IntentReviewWork),
    ActivityInitialization(ActivityInitializationWork),
    AttemptResolved(AttemptResolved),
    ActivityAdvance(ActivityAdvanceWork),
}

impl LifecycleWork {
    /// Returns the stable within-lane variant tag.
    #[must_use]
    pub const fn canonical_tag(self) -> u32 {
        match self {
            Self::EvidenceDelivery(_) => 0,
            Self::Appraisal(_) => 1,
            Self::IntentReview(_) => 2,
            Self::ActivityInitialization(_) => 3,
            Self::AttemptResolved(_) => 4,
            Self::ActivityAdvance(_) => 5,
        }
    }

    /// Returns the exact delivery moment.
    #[must_use]
    pub const fn due(self) -> SimMoment {
        match self {
            Self::EvidenceDelivery(work) => work.due(),
            Self::Appraisal(work) => work.due(),
            Self::IntentReview(work) => work.due(),
            Self::ActivityInitialization(work) => work.due(),
            Self::AttemptResolved(work) => work.due(),
            Self::ActivityAdvance(work) => work.due(),
        }
    }
}

#[cfg(test)]
mod tests {
    use world_core::{EntityId, Microstep, SimTime};
    use world_model::{
        ContainmentAppraisal, ContainmentTransferDelta, EvidenceDeliveryGeneration, PhysicalEvent,
    };

    use super::*;

    fn moment(microstep: u64) -> SimMoment {
        SimMoment::new(SimTime::from_ticks(3), Microstep::new(microstep))
    }

    fn evidence() -> EvidenceRecord {
        let actor = ActorId::from_bytes([0x11; 32]);
        let delta = ContainmentTransferDelta::new(
            actor,
            EntityId::from_bytes([0x12; 32]),
            EntityId::from_bytes([0x13; 32]),
            EntityId::from_bytes([0x14; 32]),
        )
        .unwrap_or_else(|error| panic!("fixture transfer must be valid: {error}"));
        let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
            unreachable!("transfer constructor must produce a transfer event");
        };
        EvidenceRecord::direct_item_transfer(
            actor,
            EvidenceDeliveryGeneration::new(1)
                .unwrap_or_else(|| panic!("one is a valid evidence generation")),
            event,
        )
    }

    #[test]
    fn lifecycle_work_variant_tags_are_stable_and_complete() {
        let actor = ActorId::from_bytes([0x21; 32]);
        let generation = LifecycleGeneration::new(7);
        let due = moment(4);
        let evidence = evidence();
        let work = [
            LifecycleWork::EvidenceDelivery(EvidenceDeliveryWork::new(evidence, due)),
            LifecycleWork::Appraisal(AppraisalWork::new(actor, generation, due)),
            LifecycleWork::IntentReview(IntentReviewWork::new(actor, generation, due)),
            LifecycleWork::ActivityInitialization(ActivityInitializationWork::new(
                actor, generation, due,
            )),
            LifecycleWork::AttemptResolved(AttemptResolved::new(
                ActionOpportunityId::from_bytes([0x22; 32]),
                due,
            )),
            LifecycleWork::ActivityAdvance(ActivityAdvanceWork::new(actor, generation, due)),
        ];

        assert_eq!(work.map(LifecycleWork::canonical_tag), [0, 1, 2, 3, 4, 5]);
        assert!(work.into_iter().all(|work| work.due() == due));
    }

    #[test]
    fn coalescing_roles_have_stable_tags() {
        assert_eq!(
            [
                LifecycleRole::Appraisal,
                LifecycleRole::IntentReview,
                LifecycleRole::ActivityInitialization,
                LifecycleRole::ActivityAdvance,
            ]
            .map(LifecycleRole::canonical_tag),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn coalescing_roles_accept_only_their_concrete_causes() {
        let actor = ActorId::from_bytes([0x31; 32]);
        let evidence = evidence();
        let appraisal = ContainmentAppraisal::new(
            actor,
            EntityId::from_bytes([0x32; 32]),
            EntityId::from_bytes([0x33; 32]),
            EntityId::from_bytes([0x34; 32]),
            evidence.id(),
        )
        .material_fingerprint();
        let causes = [
            (
                LifecycleRole::Appraisal,
                LifecycleCause::Evidence(evidence.id()),
            ),
            (
                LifecycleRole::IntentReview,
                LifecycleCause::Appraisal {
                    generation: LifecycleGeneration::new(2),
                    material: appraisal,
                },
            ),
            (
                LifecycleRole::ActivityInitialization,
                LifecycleCause::Intent {
                    intent: IntentId::from_bytes([0x35; 32]),
                    version: IntentVersion::INITIAL,
                },
            ),
            (
                LifecycleRole::ActivityAdvance,
                LifecycleCause::AttemptResolved(ActionOpportunityId::from_bytes([0x37; 32])),
            ),
        ];

        assert!(causes.into_iter().all(|(role, cause)| role.accepts(cause)));
        assert!(!LifecycleRole::IntentReview.accepts(LifecycleCause::Evidence(evidence.id())));
    }
}
