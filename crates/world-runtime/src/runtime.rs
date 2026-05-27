use world_core::{AuthorityClass, CausalTransactionIdIssuer, EventRecordIdIssuer};
use world_defs::DefinitionRegistry;
use world_model::{InvalidationPackage, InvalidationSource, WorldModel};

use crate::{
    RuntimeError,
    commit::CommitFinalizer,
    effects::TypedEffectInterpreter,
    outcome::{CommittedOutcome, RejectedOutcome, RejectionReason, RuntimeOutcome},
    request::RuntimeRequest,
    transaction::{CausalTransactionBuilder, CausalTransactionHeader, EffectStager},
    validation::{RuntimeValidationFailure, RuntimeValidator},
};

/// Public causal mutation waist.
pub struct CausalRuntime {
    definitions: DefinitionRegistry,
    transaction_ids: CausalTransactionIdIssuer,
    event_ids: EventRecordIdIssuer,
    interpreter: TypedEffectInterpreter,
}

impl CausalRuntime {
    /// Creates a causal runtime with fresh id issuers.
    pub fn new(definitions: DefinitionRegistry) -> Self {
        Self {
            definitions,
            transaction_ids: CausalTransactionIdIssuer::new(),
            event_ids: EventRecordIdIssuer::new(),
            interpreter: TypedEffectInterpreter,
        }
    }

    /// Creates a causal runtime with explicit id issuers.
    pub fn with_issuers(
        definitions: DefinitionRegistry,
        transaction_ids: CausalTransactionIdIssuer,
        event_ids: EventRecordIdIssuer,
    ) -> Self {
        Self {
            definitions,
            transaction_ids,
            event_ids,
            interpreter: TypedEffectInterpreter,
        }
    }

    /// Executes one runtime request against the model through the hard commit waist.
    pub fn execute(
        &mut self,
        model: &mut WorldModel,
        request: RuntimeRequest,
    ) -> Result<RuntimeOutcome, RuntimeError> {
        let action_id = request.action();
        let Some(action) = self.definitions.action(action_id) else {
            return Ok(RuntimeOutcome::Rejected(RejectedOutcome::new(
                action_id,
                RejectionReason::UnknownAction { action: action_id },
            )));
        };

        let bound = match request.bind(action) {
            Ok(bound) => bound,
            Err(rejected) => return Ok(RuntimeOutcome::Rejected(rejected)),
        };

        let Some(program) = self.definitions.effect_program(bound.effect_program()) else {
            return Err(RuntimeError::MissingEffectProgram {
                action: action.id(),
                effect_program: bound.effect_program(),
            });
        };

        if let Err(failure) = RuntimeValidator::validate(model, action, &bound, program) {
            return match failure {
                RuntimeValidationFailure::Rejected(rejected) => {
                    Ok(RuntimeOutcome::Rejected(rejected))
                }
                RuntimeValidationFailure::Runtime(error) => Err(error),
            };
        }

        let Some(transaction_id) = self.transaction_ids.issue() else {
            return Err(RuntimeError::TransactionIdExhausted);
        };

        let mut invalidation =
            InvalidationPackage::new(InvalidationSource::HardCommit(transaction_id));
        invalidation.mark_authority_class(AuthorityClass::Hard);

        let mut transaction = CausalTransactionBuilder::new(
            CausalTransactionHeader {
                id: transaction_id,
                source: bound.source(),
                action: bound.action(),
                effect_program: bound.effect_program(),
                occurred_at: bound.submitted_at(),
                replay_level: program.replay_level(),
                provenance: bound.provenance(),
            },
            invalidation,
        );

        {
            let mut stager = EffectStager::new(model, &mut transaction);
            self.interpreter
                .interpret(program, &bound, &mut stager, &mut self.event_ids)?;
        }

        let accepted = CommitFinalizer::finalize(transaction, program)?;
        let event_ids = accepted
            .events()
            .iter()
            .map(|event| event.id())
            .collect::<Vec<_>>();
        let application = model.apply_hard_commit(accepted)?;

        Ok(RuntimeOutcome::Committed(CommittedOutcome::new(
            transaction_id,
            event_ids,
            application.invalidation(),
        )))
    }
}
