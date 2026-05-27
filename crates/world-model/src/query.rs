use world_core::{ActorId, AuthorityClass};

use crate::{AuthorityRead, EpistemicHolder, EpistemicRecord, WorldModel};

/// Entry point for model read surfaces.
#[derive(Clone, Copy, Debug)]
pub struct QueryLayer<'model> {
    model: &'model WorldModel,
}

impl<'model> QueryLayer<'model> {
    pub(crate) const fn new(model: &'model WorldModel) -> Self {
        Self { model }
    }

    /// Returns the privileged kernel read surface.
    pub const fn kernel(self) -> KernelQuery<'model> {
        KernelQuery { model: self.model }
    }

    /// Returns an actor-relative read surface.
    pub const fn actor_relative(self, actor: ActorId) -> ActorRelativeQuery<'model> {
        ActorRelativeQuery {
            model: self.model,
            actor,
        }
    }

    /// Returns a semantic-context read surface.
    pub const fn semantic_context(self, actor: Option<ActorId>) -> SemanticContextQuery<'model> {
        SemanticContextQuery {
            model: self.model,
            actor,
        }
    }

    /// Returns an omniscient debug read surface.
    pub const fn debug(self) -> DebugQuery<'model> {
        DebugQuery { model: self.model }
    }
}

/// Privileged read surface for validation and runtime kernels.
#[derive(Clone, Copy, Debug)]
pub struct KernelQuery<'model> {
    model: &'model WorldModel,
}

impl<'model> KernelQuery<'model> {
    /// Returns authority labels for this query surface.
    pub fn read_labels(self) -> impl Iterator<Item = AuthorityRead> {
        [
            AuthorityRead::hard_world(),
            AuthorityRead::hard_relation(),
            AuthorityRead::event_history(),
            AuthorityRead::runtime_control(),
        ]
        .into_iter()
    }

    /// Returns current hard entity count.
    pub fn entity_count(self) -> usize {
        self.model.world_store().len()
    }

    /// Returns hard relation count.
    pub fn hard_relation_count(self) -> usize {
        self.model
            .relation_store()
            .count_by_authority(AuthorityClass::Hard)
    }

    /// Returns committed event count.
    pub fn committed_event_count(self) -> usize {
        self.model.event_history().event_count()
    }

    /// Returns runtime-control record count.
    pub fn runtime_control_record_count(self) -> usize {
        self.model.runtime_control_store().len()
    }
}

/// Actor-scoped read surface.
#[derive(Clone, Copy, Debug)]
pub struct ActorRelativeQuery<'model> {
    model: &'model WorldModel,
    actor: ActorId,
}

impl<'model> ActorRelativeQuery<'model> {
    /// Returns the actor scope for this read surface.
    pub const fn actor(self) -> ActorId {
        self.actor
    }

    /// Returns authority labels for this query surface.
    pub fn read_labels(self) -> impl Iterator<Item = AuthorityRead> {
        [
            AuthorityRead::epistemic_store(),
            AuthorityRead::derived_view(AuthorityClass::ActorTruth),
        ]
        .into_iter()
    }

    /// Returns actor-owned epistemic record count visible to this surface.
    pub fn epistemic_record_count(self) -> usize {
        self.model
            .epistemic_store()
            .count_for_holder(EpistemicHolder::Actor(self.actor))
    }

    /// Iterates actor-owned epistemic records visible to this surface.
    pub fn epistemic_records(self) -> impl Iterator<Item = &'model EpistemicRecord> {
        self.model
            .epistemic_store()
            .records_for_holder(EpistemicHolder::Actor(self.actor))
    }
}

/// Read surface for semantic context assembly.
#[derive(Clone, Copy, Debug)]
pub struct SemanticContextQuery<'model> {
    model: &'model WorldModel,
    actor: Option<ActorId>,
}

impl<'model> SemanticContextQuery<'model> {
    /// Returns the actor scope, if the semantic context is actor-relative.
    pub const fn actor(self) -> Option<ActorId> {
        self.actor
    }

    /// Returns authority labels for this query surface.
    pub fn read_labels(self) -> impl Iterator<Item = AuthorityRead> {
        [
            AuthorityRead::social_store(),
            AuthorityRead::chronology_store(),
            AuthorityRead::epistemic_store(),
            AuthorityRead::appraisal_store(),
        ]
        .into_iter()
    }

    /// Returns accepted social record count.
    pub fn social_record_count(self) -> usize {
        self.model.social_store().len()
    }

    /// Returns accepted chronology record count.
    pub fn chronology_record_count(self) -> usize {
        self.model.chronology_store().len()
    }

    /// Returns holder-relative record count.
    pub fn epistemic_record_count(self) -> usize {
        self.model.epistemic_store().len()
    }

    /// Returns accepted appraisal record count.
    pub fn appraisal_record_count(self) -> usize {
        self.model.appraisal_store().len()
    }
}

/// Omniscient debug read surface.
#[derive(Clone, Copy, Debug)]
pub struct DebugQuery<'model> {
    model: &'model WorldModel,
}

impl<'model> DebugQuery<'model> {
    /// Returns whether this surface is explicitly omniscient.
    pub const fn is_omniscient(self) -> bool {
        true
    }

    /// Returns authority labels for this query surface.
    pub fn read_labels(self) -> impl Iterator<Item = AuthorityRead> {
        [
            AuthorityRead::hard_world(),
            AuthorityRead::hard_relation(),
            AuthorityRead::social_relation(),
            AuthorityRead::event_history(),
            AuthorityRead::runtime_control(),
            AuthorityRead::social_store(),
            AuthorityRead::chronology_store(),
            AuthorityRead::epistemic_store(),
            AuthorityRead::appraisal_store(),
        ]
        .into_iter()
    }

    /// Returns the total number of stored authority records visible to debug.
    pub fn total_record_count(self) -> usize {
        self.model.world_store().len()
            + self.model.relation_store().len()
            + self.model.event_history().transaction_count()
            + self.model.event_history().event_count()
            + self.model.runtime_control_store().len()
            + self.model.social_store().len()
            + self.model.chronology_store().len()
            + self.model.epistemic_store().len()
            + self.model.appraisal_store().len()
    }
}
