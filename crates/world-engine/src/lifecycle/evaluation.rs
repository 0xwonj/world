use std::collections::BTreeSet;

use world_context::{
    ActivityEvaluationCause, ActivityProjector, ContainmentActivityProjector,
    ContainmentAppraisalProjector, ContainmentIntentProjector, EvidenceAssimilationPayload,
};
use world_core::{ActorId, EntityId};
use world_decision::{ActivityController, AppraisalEvaluator, EvidenceAssimilator, IntentPolicy};
use world_model::{
    ActionSponsor, Activity, ActivityGeneration, ContainmentAppraisal, EvidenceRecord, Intent,
    IntentGeneration, WorldSnapshot,
};
use world_runtime::{
    ActivityAdvanceResult, ActivityInitializationResult, AppraisalResult, IntentReviewResult,
    MomentWorkDecision, MomentWorkInput, RuntimeEvaluationError,
};

use super::{
    ActivityCoordinator, AppraisalCoordinator, CoordinatedActivityAdvancement,
    CoordinatedActivityInitialization, CoordinatedAppraisal, CoordinatedIntentReview,
    EvidenceCoordinator, IntentCoordinator,
};

pub(crate) fn assimilate_evidence(
    input: MomentWorkInput<'_>,
    snapshot: &WorldSnapshot,
    actor: ActorId,
    evidence: &[EvidenceRecord],
    assimilator: &dyn EvidenceAssimilator,
) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
    let payload = EvidenceAssimilationPayload::new(
        actor,
        snapshot.accepted().epistemic().actor_version(actor),
        evidence.to_vec(),
        assimilator.semantics_id(),
    )
    .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let coordinated =
        EvidenceCoordinator::coordinate(snapshot.accepted().epistemic(), &payload, assimilator)
            .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let (_, coordinated_actor, expected_version, successor) = coordinated.into_parts();
    if coordinated_actor != actor
        || expected_version != snapshot.accepted().epistemic().actor_version(actor)
    {
        return Err(RuntimeEvaluationError::Integrity);
    }
    MomentWorkDecision::assimilate_evidence(input, successor)
        .map_err(|_| RuntimeEvaluationError::Integrity)
}

pub(crate) fn appraise_containment(
    input: MomentWorkInput<'_>,
    snapshot: &WorldSnapshot,
    actor: ActorId,
    evidence: &[EvidenceRecord],
    previous: &[ContainmentAppraisal],
    evaluator: &dyn AppraisalEvaluator,
) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
    let items = containment_evidence_items(actor, evidence)?;
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let previous = previous
            .iter()
            .find(|appraisal| appraisal.actor() == actor && appraisal.item() == item)
            .copied();
        let payload = ContainmentAppraisalProjector::new()
            .build(snapshot, actor, item, previous, evaluator.semantics_id())
            .map_err(|_| RuntimeEvaluationError::Integrity)?;
        let coordinated = AppraisalCoordinator::coordinate(&payload, evaluator)
            .map_err(|_| RuntimeEvaluationError::Integrity)?;
        match coordinated {
            CoordinatedAppraisal::Present {
                appraisal,
                material_changed,
                ..
            } => results.push(AppraisalResult::present(appraisal, material_changed)),
            CoordinatedAppraisal::NoChange { .. } => {}
            CoordinatedAppraisal::Retract {
                before,
                supporting_evidence,
                ..
            } => results.push(AppraisalResult::retract(before, supporting_evidence)),
        }
    }
    MomentWorkDecision::publish_appraisals(input, results)
        .map_err(|_| RuntimeEvaluationError::Integrity)
}

fn containment_evidence_items(
    actor: ActorId,
    evidence: &[EvidenceRecord],
) -> Result<BTreeSet<EntityId>, RuntimeEvaluationError> {
    let mut items = BTreeSet::new();
    for record in evidence {
        if record.observer() != actor {
            return Err(RuntimeEvaluationError::Integrity);
        }
        if let Some(item) = record.provenance().containment_item() {
            items.insert(item);
        }
    }
    Ok(items)
}

pub(crate) fn review_intent(
    input: MomentWorkInput<'_>,
    snapshot: &WorldSnapshot,
    actor: ActorId,
    generation: u64,
    appraisals: &[ContainmentAppraisal],
    policy: &dyn IntentPolicy,
) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
    let appraisal = focal_appraisal(actor, appraisals)?;
    let projector = ContainmentIntentProjector::new();
    let build = projector
        .build(snapshot, appraisal, policy.semantics_id())
        .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let generation = IntentGeneration::new(generation).ok_or(RuntimeEvaluationError::Integrity)?;
    let coordinated =
        IntentCoordinator::coordinate(snapshot.accepted().agency(), build, generation, policy)
            .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let result = match coordinated {
        CoordinatedIntentReview::Adopt { intent, .. } => IntentReviewResult::Adopt(intent),
        CoordinatedIntentReview::NoChange { .. } => IntentReviewResult::NoChange,
    };
    MomentWorkDecision::review_intent(input, result).map_err(|_| RuntimeEvaluationError::Integrity)
}

fn focal_appraisal(
    actor: ActorId,
    appraisals: &[ContainmentAppraisal],
) -> Result<ContainmentAppraisal, RuntimeEvaluationError> {
    appraisals
        .iter()
        .copied()
        .filter(|appraisal| appraisal.actor() == actor)
        .min_by_key(|appraisal| {
            (
                appraisal.material_fingerprint(),
                appraisal.item(),
                appraisal.supporting_evidence(),
            )
        })
        .ok_or(RuntimeEvaluationError::Integrity)
}

pub(crate) fn initialize_activity(
    input: MomentWorkInput<'_>,
    snapshot: &WorldSnapshot,
    actor: ActorId,
    generation: u64,
    intents: &[Intent],
    controller: &dyn ActivityController,
) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
    let intent = intents
        .iter()
        .copied()
        .filter(|intent| intent.actor() == actor)
        .min_by_key(|intent| intent.id())
        .ok_or(RuntimeEvaluationError::Integrity)?;
    let payload = ContainmentActivityProjector::new()
        .initialization(
            snapshot,
            intent.id(),
            ActivityEvaluationCause::ScheduledRecovery,
            controller.semantics_id(),
        )
        .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let generation =
        ActivityGeneration::new(generation).ok_or(RuntimeEvaluationError::Integrity)?;
    let coordinated = ActivityCoordinator::initialize(
        snapshot.accepted().agency(),
        &payload,
        generation,
        controller,
    )
    .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let result = match coordinated {
        CoordinatedActivityInitialization::Start {
            activity,
            opportunity,
            ..
        } => ActivityInitializationResult::Start {
            activity,
            opportunity,
        },
        CoordinatedActivityInitialization::TransitionIntent {
            expected_version,
            successor,
            ..
        } => ActivityInitializationResult::TransitionIntent {
            expected_version,
            successor,
        },
    };
    MomentWorkDecision::initialize_activity(input, result)
        .map_err(|_| RuntimeEvaluationError::Integrity)
}

pub(crate) fn advance_activity(
    input: MomentWorkInput<'_>,
    snapshot: &WorldSnapshot,
    actor: ActorId,
    activities: &[Activity],
    attempted: &[world_model::ActionOpportunity],
    controller: &dyn ActivityController,
) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
    let (activity, cause) = focused_activity(snapshot, actor, activities, attempted)?;
    let payload = ActivityProjector::new()
        .advancement(snapshot, activity.id(), cause, controller.semantics_id())
        .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let coordinated = ActivityCoordinator::advance(
        snapshot.accepted().agency(),
        &payload,
        attempted,
        controller,
    )
    .map_err(|_| RuntimeEvaluationError::Integrity)?;
    let result = match coordinated {
        CoordinatedActivityAdvancement::OpenAction {
            expected_version,
            successor,
            opportunity,
            ..
        } => ActivityAdvanceResult::OpenAction {
            expected_version,
            successor,
            opportunity,
        },
        CoordinatedActivityAdvancement::Transition {
            expected_version,
            successor,
            ..
        } => ActivityAdvanceResult::Transition {
            expected_version,
            successor,
        },
        CoordinatedActivityAdvancement::Terminal {
            expected_activity_version,
            activity_successor,
            expected_intent_version,
            intent_successor,
            ..
        } => ActivityAdvanceResult::Terminal {
            expected_activity_version,
            activity_successor,
            expected_intent_version,
            intent_successor,
        },
        CoordinatedActivityAdvancement::NoChange {
            activity,
            expected_version,
            ..
        } => ActivityAdvanceResult::NoChange {
            activity,
            expected_version,
        },
    };
    MomentWorkDecision::advance_activity(input, result)
        .map_err(|_| RuntimeEvaluationError::Integrity)
}

fn focused_activity(
    snapshot: &WorldSnapshot,
    actor: ActorId,
    activities: &[Activity],
    attempted: &[world_model::ActionOpportunity],
) -> Result<(Activity, ActivityEvaluationCause), RuntimeEvaluationError> {
    let agency = snapshot.accepted().agency();
    let focused = agency
        .focused_activity(actor)
        .ok_or(RuntimeEvaluationError::Integrity)?;
    let activity = agency
        .activity(focused)
        .copied()
        .filter(|activity| activity.actor() == actor)
        .ok_or(RuntimeEvaluationError::Integrity)?;
    let retained_activity = activities.contains(&activity);
    let attempted_action = attempted.iter().any(|opportunity| {
        opportunity.actor() == actor
            && matches!(
                opportunity.sponsor(),
                ActionSponsor::Activity(sponsor)
                    if sponsor.activity() == focused
                        && sponsor.expected_version() == activity.version()
            )
    });
    if attempted_action {
        Ok((activity, ActivityEvaluationCause::AttemptedAction))
    } else if retained_activity {
        Ok((activity, ActivityEvaluationCause::ScheduledRecovery))
    } else {
        Err(RuntimeEvaluationError::Integrity)
    }
}

#[cfg(test)]
mod tests {
    use world_core::EntityId;
    use world_model::{
        ContainmentAppraisal, ContainmentTransferDelta, EvidenceDeliveryGeneration, PhysicalEvent,
        RelocationProcessId,
    };

    use super::*;

    fn actor(byte: u8) -> ActorId {
        ActorId::from_bytes([byte; 32])
    }

    fn entity(byte: u8) -> EntityId {
        EntityId::from_bytes([byte; 32])
    }

    fn evidence(observer: ActorId, generation: u64, item: EntityId) -> EvidenceRecord {
        let delta = ContainmentTransferDelta::new(observer, item, entity(0x31), entity(0x41))
            .unwrap_or_else(|error| panic!("test transfer must be valid: {error}"));
        let PhysicalEvent::ItemTransferred(event) = PhysicalEvent::item_transferred(delta) else {
            unreachable!("item transfer constructor returned another event family")
        };
        EvidenceRecord::direct_item_transfer(
            observer,
            EvidenceDeliveryGeneration::new(generation)
                .unwrap_or_else(|| panic!("test generation is nonzero")),
            event,
        )
    }

    #[test]
    fn containment_appraisal_subjects_are_unique_and_canonical() {
        let observer = actor(0x10);
        let first = entity(0x20);
        let second = entity(0x21);
        let items = containment_evidence_items(
            observer,
            &[
                evidence(observer, 2, second),
                evidence(observer, 1, first),
                evidence(observer, 3, second),
            ],
        )
        .unwrap_or_else(|error| panic!("evidence subjects must be valid: {error:?}"));

        assert_eq!(items.into_iter().collect::<Vec<_>>(), [first, second]);
    }

    #[test]
    fn relocation_evidence_has_no_containment_appraisal_subject() {
        let observer = actor(0x10);
        let departure = PhysicalEvent::actor_departed(
            RelocationProcessId::from_bytes([0x51; 32]),
            observer,
            entity(0x30),
            entity(0x40),
        );
        let evidence = EvidenceRecord::direct_physical_event(
            observer,
            EvidenceDeliveryGeneration::new(1)
                .unwrap_or_else(|| panic!("test generation is nonzero")),
            departure,
        );

        assert_eq!(
            containment_evidence_items(observer, &[evidence]),
            Ok(BTreeSet::new())
        );
    }

    #[test]
    fn focal_appraisal_is_independent_of_cause_order() {
        let observer = actor(0x10);
        let first_evidence = evidence(observer, 1, entity(0x20));
        let second_evidence = evidence(observer, 2, entity(0x21));
        let first = ContainmentAppraisal::new(
            observer,
            entity(0x20),
            entity(0x40),
            entity(0x30),
            first_evidence.id(),
        );
        let second = ContainmentAppraisal::new(
            observer,
            entity(0x21),
            entity(0x41),
            entity(0x31),
            second_evidence.id(),
        );

        assert_eq!(
            focal_appraisal(observer, &[first, second]),
            focal_appraisal(observer, &[second, first])
        );
    }
}
