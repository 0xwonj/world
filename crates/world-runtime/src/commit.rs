use std::collections::BTreeSet;

use world_core::{AuthorityClass, DefinitionId};
use world_defs::{EffectProgramDef, EventRecordSpec};
use world_model::{AcceptedHardCommit, EventCommit, StoreFamily, TransactionCommit};

use crate::{RuntimeError, transaction::CausalTransactionBuilder};

pub(crate) struct CommitFinalizer;

impl CommitFinalizer {
    pub(crate) fn finalize(
        transaction: CausalTransactionBuilder,
        program: &EffectProgramDef,
    ) -> Result<AcceptedHardCommit, RuntimeError> {
        let emitted = transaction.emitted_event_specs();
        validate_required_events(
            program.id(),
            program.event_contract().required_events(),
            &emitted,
        )?;

        let mut staged = transaction.into_parts();

        staged
            .invalidation
            .mark_authority_class(AuthorityClass::Hard);
        staged
            .invalidation
            .mark_store_family(StoreFamily::EventHistory);

        let transaction = TransactionCommit::new(
            staged.id,
            staged.source.into(),
            staged.action,
            staged.effect_program,
            staged.replay_level,
            staged.occurred_at,
            staged.provenance,
        );
        let events = staged.events.into_iter().map(|event| {
            EventCommit::new(
                event.id(),
                event.spec().clone(),
                event.roles().to_vec(),
                staged.occurred_at,
                event.provenance(),
            )
        });

        AcceptedHardCommit::new(transaction, events, staged.changes, staged.invalidation)
            .map_err(RuntimeError::from)
    }
}

fn validate_required_events<'a>(
    effect_program: DefinitionId,
    required: impl Iterator<Item = &'a EventRecordSpec>,
    emitted: &BTreeSet<EventRecordSpec>,
) -> Result<(), RuntimeError> {
    for event in required {
        if !emitted.contains(event) {
            return Err(RuntimeError::RequiredEventMissing {
                effect_program,
                event: event.clone(),
            });
        }
    }

    Ok(())
}
