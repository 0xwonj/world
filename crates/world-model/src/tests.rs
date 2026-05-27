use std::collections::BTreeSet;

use world_core::{
    ActivityId, ActivityIdIssuer, ActorId, AuthorityClass, CausalSource, CausalTransactionId,
    CausalTransactionIdIssuer, DefinitionId, EntityId, EventRecordId, EventRecordIdIssuer,
    ProcessInstanceId, ProcessInstanceIdIssuer, ProvenanceKey, QueryEpoch, ReplayLevel,
    ReservationId, ReservationIdIssuer, SimulationTime, VersionAnchor,
};
use world_defs::{EventKind, EventRecordSpec, RoleName};

use super::*;

fn entity(value: u64) -> EntityId {
    let Some(value) = EntityId::new(value) else {
        panic!("test entity id must be nonzero");
    };
    value
}

fn actor(value: u64) -> ActorId {
    let Some(value) = ActorId::new(value) else {
        panic!("test actor id must be nonzero");
    };
    value
}

fn definition(value: u64) -> DefinitionId {
    let Some(value) = DefinitionId::new(value) else {
        panic!("test definition id must be nonzero");
    };
    value
}

fn provenance(value: u64) -> ProvenanceKey {
    let Some(value) = ProvenanceKey::new(value) else {
        panic!("test provenance key must be nonzero");
    };
    value
}

fn version(value: u64) -> VersionAnchor {
    let Some(value) = VersionAnchor::new(value) else {
        panic!("test version anchor must be nonzero");
    };
    value
}

fn role_name(value: &'static str) -> RoleName {
    let Some(value) = RoleName::new(value) else {
        panic!("test role name must be non-empty");
    };
    value
}

fn event_kind(value: &'static str) -> EventKind {
    let Some(value) = EventKind::new(value) else {
        panic!("test event kind must be non-empty");
    };
    value
}

fn accepted_record(value: u64) -> AcceptedRecordId {
    let Some(value) = AcceptedRecordId::new(value) else {
        panic!("test accepted record id must be nonzero");
    };
    value
}

fn derived_view(value: u64) -> DerivedViewKey {
    let Some(value) = DerivedViewKey::new(value) else {
        panic!("test derived view key must be nonzero");
    };
    value
}

fn transaction(value: u64) -> CausalTransactionId {
    let Ok(mut issuer) = CausalTransactionIdIssuer::starting_at(value) else {
        panic!("test transaction id must be nonzero");
    };
    let Some(value) = issuer.issue() else {
        panic!("test transaction id space must not be exhausted");
    };
    value
}

fn event(value: u64) -> EventRecordId {
    let Ok(mut issuer) = EventRecordIdIssuer::starting_at(value) else {
        panic!("test event id must be nonzero");
    };
    let Some(value) = issuer.issue() else {
        panic!("test event id space must not be exhausted");
    };
    value
}

fn process(value: u64) -> ProcessInstanceId {
    let Ok(mut issuer) = ProcessInstanceIdIssuer::starting_at(value) else {
        panic!("test process id must be nonzero");
    };
    let Some(value) = issuer.issue() else {
        panic!("test process id space must not be exhausted");
    };
    value
}

fn activity(value: u64) -> ActivityId {
    let Ok(mut issuer) = ActivityIdIssuer::starting_at(value) else {
        panic!("test activity id must be nonzero");
    };
    let Some(value) = issuer.issue() else {
        panic!("test activity id space must not be exhausted");
    };
    value
}

fn reservation(value: u64) -> ReservationId {
    let Ok(mut issuer) = ReservationIdIssuer::starting_at(value) else {
        panic!("test reservation id must be nonzero");
    };
    let Some(value) = issuer.issue() else {
        panic!("test reservation id space must not be exhausted");
    };
    value
}

fn must_ok<T>(result: Result<T, ModelError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected model error: {error}"),
    }
}

fn must_some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected value to be present"),
    }
}

fn event_spec() -> EventRecordSpec {
    let Ok(spec) = EventRecordSpec::new(
        event_kind("EntityTransferred"),
        [
            role_name("actor"),
            role_name("item"),
            role_name("destination"),
        ],
        version(1),
    ) else {
        panic!("test event spec must be valid");
    };
    spec
}

fn event_roles() -> Vec<EventRoleBinding> {
    vec![
        EventRoleBinding::new(role_name("actor"), entity(1)),
        EventRoleBinding::new(role_name("item"), entity(2)),
        EventRoleBinding::new(role_name("destination"), entity(3)),
    ]
}

fn transaction_record(
    id: CausalTransactionId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
) -> TransactionRecord {
    TransactionRecord::new(
        id,
        CausalSource::Tooling,
        definition(100),
        definition(101),
        ReplayLevel::EventRebuild,
        occurred_at,
        provenance,
    )
}

fn transaction_commit(
    id: CausalTransactionId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
) -> TransactionCommit {
    TransactionCommit::new(
        id,
        CausalSource::Tooling,
        definition(100),
        definition(101),
        ReplayLevel::EventRebuild,
        occurred_at,
        provenance,
    )
}

fn event_commit(
    id: EventRecordId,
    occurred_at: SimulationTime,
    provenance: Option<ProvenanceKey>,
) -> EventCommit {
    EventCommit::new(id, event_spec(), event_roles(), occurred_at, provenance)
}

fn hard_invalidation(
    transaction: CausalTransactionId,
    stores: impl IntoIterator<Item = StoreFamily>,
) -> InvalidationPackage {
    let mut invalidation = InvalidationPackage::new(InvalidationSource::HardCommit(transaction));
    invalidation
        .mark_authority_class(AuthorityClass::Hard)
        .mark_store_family(StoreFamily::EventHistory);
    for store in stores {
        invalidation.mark_store_family(store);
    }
    invalidation
}

#[test]
fn empty_model_has_read_only_query_surfaces() {
    let model = WorldModel::new();

    assert!(model.world_store().is_empty());
    assert!(model.relation_store().is_empty());
    assert!(model.event_history().is_empty());
    assert!(model.runtime_control_store().is_empty());
    assert!(model.derived_view_registry().is_empty());

    let query = model.query_layer();
    assert_eq!(query.kernel().entity_count(), 0);
    assert!(query.debug().is_omniscient());
    assert_eq!(query.debug().total_record_count(), 0);

    let actor_query = query.actor_relative(actor(1));
    assert_eq!(actor_query.actor(), actor(1));
    assert!(
        actor_query
            .read_labels()
            .all(|label| label.authority_class() == AuthorityClass::ActorTruth)
    );
}

#[test]
fn authority_read_constructors_match_store_authority() {
    let cases = [
        (
            AuthorityRead::hard_world(),
            AuthorityClass::Hard,
            StoreFamily::World,
        ),
        (
            AuthorityRead::event_history(),
            AuthorityClass::Hard,
            StoreFamily::EventHistory,
        ),
        (
            AuthorityRead::runtime_control(),
            AuthorityClass::RuntimeControl,
            StoreFamily::RuntimeControl,
        ),
        (
            AuthorityRead::social_store(),
            AuthorityClass::Social,
            StoreFamily::SocialInstitutional,
        ),
        (
            AuthorityRead::chronology_store(),
            AuthorityClass::Chronology,
            StoreFamily::Chronology,
        ),
        (
            AuthorityRead::epistemic_store(),
            AuthorityClass::ActorTruth,
            StoreFamily::Epistemic,
        ),
        (
            AuthorityRead::appraisal_store(),
            AuthorityClass::Appraisal,
            StoreFamily::AppraisalRecord,
        ),
        (
            AuthorityRead::hard_relation(),
            AuthorityClass::Hard,
            StoreFamily::Relation,
        ),
        (
            AuthorityRead::social_relation(),
            AuthorityClass::Social,
            StoreFamily::Relation,
        ),
    ];

    for (label, authority, family) in cases {
        assert_eq!(label.authority_class(), authority);
        assert_eq!(label.store_family(), family);
    }
}

#[test]
fn store_families_keep_authority_records_separate() {
    let mut model = WorldModel::new();

    must_ok(model.insert_entity(EntitySnapshot::new(entity(1), None, Some(provenance(1)))));
    assert_eq!(
        model.insert_entity(EntitySnapshot::new(entity(1), None, None)),
        Err(ModelError::DuplicateEntity { entity: entity(1) })
    );

    must_ok(model.insert_relation(RelationRecord::new(
        entity(1),
        RelationFamily::ContainedIn,
        entity(2),
        None,
    )));
    must_ok(model.insert_relation(RelationRecord::new(
        entity(1),
        RelationFamily::MemberOf,
        entity(3),
        None,
    )));

    assert_eq!(
        model
            .relation_store()
            .count_by_authority(AuthorityClass::Hard),
        1
    );
    assert_eq!(
        model
            .relation_store()
            .count_by_authority(AuthorityClass::Social),
        1
    );

    let same_local_record_id = accepted_record(1);
    must_ok(model.insert_social_record(SocialRecord::new(
        same_local_record_id,
        Some(definition(10)),
        None,
    )));
    must_ok(model.insert_epistemic_record(EpistemicRecord::new(
        same_local_record_id,
        EpistemicHolder::Actor(actor(1)),
        Some(definition(11)),
        None,
    )));

    assert!(model.social_store().contains(same_local_record_id));
    assert!(model.epistemic_store().contains(same_local_record_id));
    assert_eq!(model.social_store().len(), 1);
    assert_eq!(model.epistemic_store().len(), 1);
}

#[test]
fn authority_family_records_have_distinct_shapes_and_reject_duplicates() {
    let mut model = WorldModel::new();
    let id = accepted_record(1);

    must_ok(model.insert_appraisal_record(AppraisalRecord::new(id, None, None)));
    assert_eq!(
        model.insert_appraisal_record(AppraisalRecord::new(id, None, None)),
        Err(ModelError::DuplicateAcceptedRecord { record: id })
    );

    let epistemic = EpistemicRecord::new(
        accepted_record(2),
        EpistemicHolder::Actor(actor(9)),
        Some(definition(99)),
        Some(provenance(99)),
    );
    assert_eq!(epistemic.holder(), EpistemicHolder::Actor(actor(9)));
    assert_eq!(epistemic.definition(), Some(definition(99)));
    assert_eq!(epistemic.provenance(), Some(provenance(99)));
}

#[test]
fn event_history_preserves_order_and_requires_known_transaction() {
    let mut model = WorldModel::new();
    let tx = transaction(1);
    let event_id = event(1);

    assert_eq!(
        model.append_event(EventRecord::new(
            event_id,
            tx,
            event_spec(),
            event_roles(),
            SimulationTime::from_ticks(10),
            None,
        )),
        Err(ModelError::MissingTransaction { transaction: tx })
    );

    let tx_cursor = must_ok(model.append_transaction(transaction_record(
        tx,
        SimulationTime::from_ticks(10),
        Some(provenance(7)),
    )));
    let event_cursor = must_ok(model.append_event(EventRecord::new(
        event_id,
        tx,
        event_spec(),
        event_roles(),
        SimulationTime::from_ticks(10),
        None,
    )));

    assert!(tx_cursor < event_cursor);
    assert_eq!(model.event_history().transaction_count(), 1);
    assert_eq!(model.event_history().event_count(), 1);
    assert_eq!(
        must_some(model.event_history().event(event_id))
            .record()
            .transaction(),
        tx
    );
    assert_eq!(
        model
            .event_history()
            .events()
            .map(|stored| stored.cursor())
            .collect::<Vec<_>>(),
        vec![event_cursor]
    );

    assert_eq!(
        model.append_event(EventRecord::new(
            event_id,
            tx,
            event_spec(),
            event_roles(),
            SimulationTime::from_ticks(11),
            None,
        )),
        Err(ModelError::DuplicateEvent { event: event_id })
    );
}

#[test]
fn accepted_hard_commit_applies_history_state_and_invalidation_together() {
    let mut model = WorldModel::new();
    let view = derived_view(10);
    must_ok(
        model.register_derived_view(must_ok(DerivedViewDescriptor::new(
            view,
            [AuthorityRead::hard_world()],
        ))),
    );

    let tx = transaction(10);
    let invalidation = hard_invalidation(tx, [StoreFamily::World, StoreFamily::Relation]);

    let commit = must_ok(AcceptedHardCommit::new(
        transaction_commit(tx, SimulationTime::from_ticks(42), Some(provenance(10))),
        [event_commit(
            event(10),
            SimulationTime::from_ticks(42),
            Some(provenance(11)),
        )],
        [
            HardStateChange::insert_entity(entity(10), None, Some(provenance(12))),
            HardStateChange::insert_relation(
                entity(10),
                RelationFamily::ContainedIn,
                entity(20),
                Some(provenance(13)),
            ),
        ],
        invalidation,
    ));

    let application = must_ok(model.apply_hard_commit(commit));

    assert_eq!(model.event_history().transaction_count(), 1);
    assert_eq!(model.event_history().event_count(), 1);
    assert!(model.world_store().contains_entity(entity(10)));
    assert!(model.relation_store().contains(RelationKey::new(
        entity(10),
        RelationFamily::ContainedIn,
        entity(20),
    )));
    assert_eq!(application.event_cursors().len(), 1);
    assert_eq!(application.invalidation().touched_views(), 1);
    assert_eq!(
        must_some(model.derived_view(view)).status(),
        DerivedViewStatus::Stale
    );
}

#[test]
fn hard_commit_rejects_non_hard_relation_without_mutating_history() {
    let mut model = WorldModel::new();
    let tx = transaction(20);
    let invalidation = InvalidationPackage::new(InvalidationSource::HardCommit(tx));
    let commit = must_ok(AcceptedHardCommit::new(
        transaction_commit(tx, SimulationTime::from_ticks(1), None),
        [event_commit(event(20), SimulationTime::from_ticks(1), None)],
        [HardStateChange::insert_relation(
            entity(1),
            RelationFamily::MemberOf,
            entity(2),
            None,
        )],
        invalidation,
    ));

    assert_eq!(
        model.apply_hard_commit(commit),
        Err(ModelError::NonHardRelationInHardCommit {
            subject: entity(1),
            family: RelationFamily::MemberOf,
            object: entity(2),
        })
    );
    assert!(model.event_history().is_empty());
    assert!(model.relation_store().is_empty());
}

#[test]
fn hard_commit_requires_invalidation_for_changed_stores() {
    let mut model = WorldModel::new();
    let tx = transaction(25);
    let mut invalidation = InvalidationPackage::new(InvalidationSource::HardCommit(tx));
    invalidation
        .mark_authority_class(AuthorityClass::Hard)
        .mark_store_family(StoreFamily::EventHistory);
    let commit = must_ok(AcceptedHardCommit::new(
        transaction_commit(tx, SimulationTime::from_ticks(1), None),
        [],
        [HardStateChange::insert_relation(
            entity(1),
            RelationFamily::ContainedIn,
            entity(2),
            None,
        )],
        invalidation,
    ));

    assert_eq!(
        model.apply_hard_commit(commit),
        Err(ModelError::MissingHardCommitStoreInvalidation {
            transaction: tx,
            store: StoreFamily::Relation,
        })
    );
    assert!(model.event_history().is_empty());
    assert!(model.relation_store().is_empty());
}

#[test]
fn hard_commit_preflight_rejects_duplicate_event_ids_without_mutating_history() {
    let mut model = WorldModel::new();
    let tx = transaction(26);
    let duplicate = event(26);
    let commit = must_ok(AcceptedHardCommit::new(
        transaction_commit(tx, SimulationTime::from_ticks(1), None),
        [
            event_commit(duplicate, SimulationTime::from_ticks(1), None),
            event_commit(duplicate, SimulationTime::from_ticks(1), None),
        ],
        [],
        hard_invalidation(tx, []),
    ));

    assert_eq!(
        model.apply_hard_commit(commit),
        Err(ModelError::DuplicateEvent { event: duplicate })
    );
    assert!(model.event_history().is_empty());
}

#[test]
fn hard_commit_application_is_atomic_when_late_storage_checks_fail() {
    let mut model = WorldModel::new();
    let relation =
        HardStateChange::insert_relation(entity(1), RelationFamily::ContainedIn, entity(2), None);

    let first_tx = transaction(30);
    let first_commit = must_ok(AcceptedHardCommit::new(
        transaction_commit(first_tx, SimulationTime::from_ticks(1), None),
        [event_commit(event(30), SimulationTime::from_ticks(1), None)],
        [relation.clone()],
        hard_invalidation(first_tx, [StoreFamily::Relation]),
    ));
    must_ok(model.apply_hard_commit(first_commit));

    let second_tx = transaction(31);
    let second_commit = must_ok(AcceptedHardCommit::new(
        transaction_commit(second_tx, SimulationTime::from_ticks(2), None),
        [event_commit(event(31), SimulationTime::from_ticks(2), None)],
        [relation],
        hard_invalidation(second_tx, [StoreFamily::Relation]),
    ));

    assert_eq!(
        model.apply_hard_commit(second_commit),
        Err(ModelError::DuplicateRelation {
            subject: entity(1),
            family: RelationFamily::ContainedIn,
            object: entity(2),
        })
    );
    assert_eq!(model.event_history().transaction_count(), 1);
    assert_eq!(model.event_history().event_count(), 1);
}

#[test]
fn runtime_control_store_is_separate_from_hard_state() {
    let mut model = WorldModel::new();
    let reservation_kind = RuntimeControlRecordKind::Reservation(reservation(1));
    let process_kind = RuntimeControlRecordKind::Process(process(1));
    let activity_kind = RuntimeControlRecordKind::Activity(activity(1));

    must_ok(model.insert_runtime_control(RuntimeControlRecord::new(
        reservation_kind,
        Some(provenance(1)),
    )));
    must_ok(model.insert_runtime_control(RuntimeControlRecord::new(process_kind, None)));
    must_ok(model.insert_runtime_control(RuntimeControlRecord::new(activity_kind, None)));

    assert_eq!(model.runtime_control_store().len(), 3);
    assert_eq!(model.world_store().len(), 0);
    assert!(model.runtime_control_store().contains(reservation_kind));
    assert_eq!(
        model.insert_runtime_control(RuntimeControlRecord::new(reservation_kind, None)),
        Err(ModelError::DuplicateRuntimeControlRecord {
            kind: reservation_kind,
        })
    );
}

#[test]
fn derived_view_registry_rejects_empty_dependencies_and_duplicates() {
    let mut model = WorldModel::new();
    let key = derived_view(1);

    assert_eq!(
        DerivedViewDescriptor::new(key, std::iter::empty::<AuthorityRead>()),
        Err(ModelError::EmptyItemField {
            type_name: "DerivedViewDescriptor",
            field: "reads",
        })
    );

    let descriptor = must_ok(DerivedViewDescriptor::new(
        key,
        [AuthorityRead::hard_world()],
    ));
    must_ok(model.register_derived_view(descriptor.clone()));
    assert_eq!(
        model.register_derived_view(descriptor),
        Err(ModelError::DuplicateDerivedView { key })
    );
}

#[test]
fn invalidation_marks_matching_views_stale_and_advances_epoch() {
    let mut model = WorldModel::new();
    let hard_view = derived_view(1);
    let actor_view = derived_view(2);

    must_ok(
        model.register_derived_view(must_ok(DerivedViewDescriptor::new(
            hard_view,
            [AuthorityRead::hard_world()],
        ))),
    );
    must_ok(
        model.register_derived_view(must_ok(DerivedViewDescriptor::new(
            actor_view,
            [AuthorityRead::epistemic_store()],
        ))),
    );

    let mut package = InvalidationPackage::new(InvalidationSource::Manual);
    package
        .mark_authority_class(AuthorityClass::Hard)
        .mark_store_family(StoreFamily::World);

    let report = must_ok(model.apply_invalidation(&package));
    assert_eq!(report.touched_views(), 1);
    assert_eq!(report.epoch(), QueryEpoch::new(1));
    assert_eq!(
        must_some(model.derived_view(hard_view)).status(),
        DerivedViewStatus::Stale
    );
    assert_eq!(
        must_some(model.derived_view(actor_view)).status(),
        DerivedViewStatus::Valid
    );

    let mut direct_package = InvalidationPackage::new(InvalidationSource::Manual);
    direct_package.mark_derived_view(actor_view);
    let report = must_ok(model.apply_invalidation(&direct_package));
    assert_eq!(report.touched_views(), 1);
    assert_eq!(
        must_some(model.derived_view(actor_view)).status(),
        DerivedViewStatus::NeedsRebuild
    );
}

#[test]
fn actor_relative_query_filters_epistemic_records_by_actor() {
    let mut model = WorldModel::new();

    must_ok(model.insert_epistemic_record(EpistemicRecord::new(
        accepted_record(1),
        EpistemicHolder::Actor(actor(1)),
        None,
        None,
    )));
    must_ok(model.insert_epistemic_record(EpistemicRecord::new(
        accepted_record(2),
        EpistemicHolder::Actor(actor(2)),
        None,
        None,
    )));

    let query = model.query_layer().actor_relative(actor(1));
    assert_eq!(query.epistemic_record_count(), 1);
    assert_eq!(
        query
            .epistemic_records()
            .map(EpistemicRecord::holder)
            .collect::<Vec<_>>(),
        vec![EpistemicHolder::Actor(actor(1))]
    );
}

#[test]
fn semantic_and_debug_queries_report_distinct_authority_labels() {
    let mut model = WorldModel::new();

    must_ok(model.insert_social_record(SocialRecord::new(accepted_record(1), None, None)));
    must_ok(model.insert_chronology_record(ChronologyRecord::new(accepted_record(1), None, None)));
    must_ok(model.insert_epistemic_record(EpistemicRecord::new(
        accepted_record(1),
        EpistemicHolder::Actor(actor(4)),
        None,
        None,
    )));
    must_ok(model.insert_appraisal_record(AppraisalRecord::new(accepted_record(1), None, None)));

    let semantic = model.query_layer().semantic_context(Some(actor(4)));
    assert_eq!(semantic.actor(), Some(actor(4)));
    assert_eq!(semantic.social_record_count(), 1);
    assert_eq!(semantic.chronology_record_count(), 1);
    assert_eq!(semantic.epistemic_record_count(), 1);
    assert_eq!(semantic.appraisal_record_count(), 1);

    let semantic_authorities = semantic
        .read_labels()
        .map(|label| label.authority_class())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        semantic_authorities,
        BTreeSet::from([
            AuthorityClass::Social,
            AuthorityClass::Chronology,
            AuthorityClass::ActorTruth,
            AuthorityClass::Appraisal,
        ])
    );

    let debug_authorities = model
        .query_layer()
        .debug()
        .read_labels()
        .map(|label| label.authority_class())
        .collect::<BTreeSet<_>>();
    assert!(debug_authorities.contains(&AuthorityClass::Hard));
    assert!(debug_authorities.contains(&AuthorityClass::RuntimeControl));
    assert!(model.query_layer().debug().is_omniscient());
}
