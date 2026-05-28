use std::collections::BTreeSet;

use world_core::{AuthorityClass, DefinitionId};
use world_defs::{EffectProgramDef, EventRecordSpec};
use world_model::{
    AcceptedHardCommit, EventCommit, StoreFamily, TransactionCause, TransactionCommit,
};

use crate::{RuntimeError, transaction::CausalTransactionBuilder};

pub(crate) struct CommitFinalizer;

impl CommitFinalizer {
    pub(crate) fn finalize_action(
        transaction: CausalTransactionBuilder,
        program: &EffectProgramDef,
    ) -> Result<AcceptedHardCommit, RuntimeError> {
        let emitted = transaction.emitted_event_specs();
        validate_required_events(
            program.id(),
            program.event_contract().required_events(),
            &emitted,
        )?;

        finalize(transaction)
    }

    pub(crate) fn finalize_eventless_process_tick(
        transaction: CausalTransactionBuilder,
    ) -> Result<AcceptedHardCommit, RuntimeError> {
        let TransactionCause::ProcessTick { .. } = transaction.cause() else {
            return Err(RuntimeError::InvalidProcessTransactionCause {
                cause: transaction.cause(),
            });
        };
        if transaction.has_staged_events() {
            return Err(RuntimeError::EventlessProcessTickEmittedEvents);
        }

        finalize(transaction)
    }
}

fn finalize(transaction: CausalTransactionBuilder) -> Result<AcceptedHardCommit, RuntimeError> {
    let mut staged = transaction.into_parts();

    staged
        .invalidation
        .mark_authority_class(AuthorityClass::Hard);
    staged
        .invalidation
        .mark_store_family(StoreFamily::EventHistory);

    let transaction = TransactionCommit::from_cause(
        staged.id,
        staged.source.into(),
        staged.cause,
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

    AcceptedHardCommit::with_control_changes(
        transaction,
        events,
        staged.changes,
        staged.control_changes,
        staged.invalidation,
    )
    .map_err(RuntimeError::from)
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
