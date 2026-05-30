use std::collections::BTreeSet;

use world_core::DefinitionId;

use crate::{
    DecisionError, DecisionExecutionMetadata, DecisionPassContract, DecisionProfile,
    DecisionRegistry, DeterminismPolicy, ImplementationMode, ProducedDecisionArtifact,
    RepresentationRole,
};

pub(crate) fn execution_metadata(
    profile: &DecisionProfile,
    pass: &DecisionPassContract,
    mode: ImplementationMode,
    metadata: &DecisionExecutionMetadata,
) -> Result<(), DecisionError> {
    if metadata.mode() != mode {
        return Err(DecisionError::ExecutionModeMismatch {
            pass: pass.id(),
            expected: mode,
            actual: metadata.mode(),
        });
    }
    if metadata.determinism() != pass.determinism() {
        return Err(DecisionError::ExecutionDeterminismMismatch {
            pass: pass.id(),
            expected: pass.determinism(),
            actual: metadata.determinism(),
        });
    }
    if pass.determinism() == DeterminismPolicy::Seeded && metadata.seed().is_none() {
        return Err(DecisionError::MissingSeedMetadata { pass: pass.id() });
    }
    if pass.determinism() != DeterminismPolicy::Seeded && metadata.seed().is_some() {
        return Err(unexpected_metadata(pass, "seed"));
    }

    let model_allowed = matches!(mode, ImplementationMode::Llm | ImplementationMode::Hybrid);
    if model_allowed && metadata.model().is_none() {
        return Err(DecisionError::MissingModelMetadata { pass: pass.id() });
    }
    if !model_allowed && metadata.model().is_some() {
        return Err(unexpected_metadata(pass, "model"));
    }

    let oracle_allowed =
        mode == ImplementationMode::Oracle || pass.determinism() == DeterminismPolicy::Oracle;
    if oracle_allowed && metadata.oracle().is_none() {
        return Err(DecisionError::MissingOracleMetadata { pass: pass.id() });
    }
    if metadata.oracle().is_some()
        && (!oracle_allowed || !profile.oracle_policy().is_oracle_labeled())
    {
        return Err(unexpected_metadata(pass, "oracle"));
    }

    if mode == ImplementationMode::Replay && metadata.replay().is_none() {
        return Err(DecisionError::MissingReplayMetadata { pass: pass.id() });
    }
    if mode != ImplementationMode::Replay && metadata.replay().is_some() {
        return Err(unexpected_metadata(pass, "replay"));
    }

    Ok(())
}

fn unexpected_metadata(pass: &DecisionPassContract, field: &'static str) -> DecisionError {
    DecisionError::UnexpectedExecutionMetadata {
        pass: pass.id(),
        field,
    }
}

pub(crate) fn executor_outputs(
    registry: &DecisionRegistry,
    pass: &DecisionPassContract,
    outputs: &[ProducedDecisionArtifact],
) -> Result<(), DecisionError> {
    let declared = pass
        .outputs()
        .iter()
        .map(|output| OutputSpec::new(output.role(), output.kind()))
        .collect::<BTreeSet<_>>();
    let mut produced = BTreeSet::new();

    for output in outputs {
        let spec = OutputSpec::new(output.role(), output.kind());
        if !declared.contains(&spec) {
            return Err(DecisionError::UndeclaredExecutorOutput {
                pass: pass.id(),
                role: output.role(),
                kind: output.kind(),
            });
        }
        if !produced.insert(spec) {
            return Err(DecisionError::DuplicateExecutorOutput {
                pass: pass.id(),
                role: output.role(),
                kind: output.kind(),
            });
        }
        let Some(representation) = registry.representation(output.kind()) else {
            return Err(DecisionError::MissingRepresentationKind {
                owner: pass.id(),
                kind: output.kind(),
            });
        };
        if !representation.can_satisfy(output.role()) {
            return Err(DecisionError::RepresentationRoleMismatch {
                owner: pass.id(),
                kind: output.kind(),
                role: output.role(),
            });
        }
    }

    for declared in declared {
        if !produced.contains(&declared) {
            return Err(DecisionError::MissingExecutorOutput {
                pass: pass.id(),
                role: declared.role,
                kind: declared.kind,
            });
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct OutputSpec {
    role: RepresentationRole,
    kind: DefinitionId,
}

impl OutputSpec {
    const fn new(role: RepresentationRole, kind: DefinitionId) -> Self {
        Self { role, kind }
    }
}
