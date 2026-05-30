use std::collections::BTreeSet;

use world_context::ContextProjectionKind;

use crate::{
    DecisionError, DecisionPassDiagnostic, DecisionRegistry, DecisionTraceBuilder,
    DecisionTraceHeader, DecisionTraceStep, DecisionTraceStepStatus, DecisionVerifierResult,
    ImplementationMode,
};

mod artifact_store;
mod context;
mod executor;
mod registry;
mod report;
mod request;
mod resolve;
mod validate;

pub use artifact_store::{
    DecisionArtifact, DecisionArtifactBody, DecisionArtifactPayload, DecisionArtifactStore,
    ProducedDecisionArtifact,
};
pub use context::{DecisionContextView, DecisionPassExecutionContext, ResolvedDecisionInput};
pub use executor::{DecisionPassDisposition, DecisionPassExecution, DecisionPassExecutor};
pub use registry::DecisionExecutorRegistry;
pub use report::{DecisionRunOutcome, DecisionRunReport};
pub use request::DecisionRunRequest;

use resolve::ExecutionFlow;

/// Executes checked decision profiles over actor-context projections.
pub struct DecisionRunner<'a> {
    registry: &'a DecisionRegistry,
    executors: &'a DecisionExecutorRegistry,
}

impl<'a> DecisionRunner<'a> {
    /// Creates a runner after validating installed executors against checked declarations.
    pub fn new(
        registry: &'a DecisionRegistry,
        executors: &'a DecisionExecutorRegistry,
    ) -> Result<Self, DecisionError> {
        executors.validate_against(registry)?;
        Ok(Self {
            registry,
            executors,
        })
    }

    /// Runs a checked profile and returns a trace-backed report.
    pub fn run(&self, request: DecisionRunRequest<'_>) -> Result<DecisionRunReport, DecisionError> {
        let Some(profile) = self.registry.profile(request.profile()) else {
            return Err(DecisionError::MissingProfile {
                profile: request.profile(),
            });
        };
        let header = DecisionTraceHeader::from_projection(
            request.actor_context(),
            profile.id(),
            profile.version(),
            profile.oracle_policy(),
        );
        let mut trace = DecisionTraceBuilder::new(header);
        let mut artifacts = DecisionArtifactStore::new();
        let mut flow = ExecutionFlow::from_context_inputs(profile.context_inputs());

        for step in profile.steps() {
            let Some(pass) = self.registry.pass(step.pass()) else {
                return Err(DecisionError::MissingPass {
                    profile: profile.id(),
                    pass: step.pass(),
                });
            };
            if step.mode() == ImplementationMode::Disabled {
                trace.push_step(DecisionTraceStep::recorded(
                    pass.id(),
                    step.mode(),
                    [],
                    [],
                    [],
                    DecisionTraceStepStatus::Skipped,
                    DecisionVerifierResult::not_run(),
                    None,
                ))?;
                continue;
            }

            let resolved = flow.resolve_pass_inputs(profile, pass)?;
            let input_refs = resolved
                .iter()
                .map(|input| input.reference())
                .collect::<Vec<_>>();
            let allowed_context = allowed_context(profile, pass);
            let actor_context = DecisionContextView::new(request.actor_context(), &allowed_context);
            let execution_context = DecisionPassExecutionContext::new(
                profile,
                pass,
                step.mode(),
                actor_context,
                &resolved,
                &artifacts,
            );
            let Some(executor) = self.executors.executor(pass.id(), step.mode()) else {
                let error = DecisionError::MissingExecutor {
                    profile: profile.id(),
                    pass: pass.id(),
                    mode: step.mode(),
                };
                push_failed_step(
                    &mut trace,
                    pass.id(),
                    step.mode(),
                    input_refs,
                    Vec::new(),
                    error,
                    None,
                )?;
                let trace = trace.fail()?;
                return Ok(DecisionRunReport::new(
                    DecisionRunOutcome::Failed,
                    trace,
                    artifacts,
                ));
            };

            let execution = match executor.execute(execution_context) {
                Ok(execution) => execution,
                Err(error) => {
                    push_failed_step(
                        &mut trace,
                        pass.id(),
                        step.mode(),
                        input_refs,
                        Vec::new(),
                        error,
                        None,
                    )?;
                    let trace = trace.fail()?;
                    return Ok(DecisionRunReport::new(
                        DecisionRunOutcome::Failed,
                        trace,
                        artifacts,
                    ));
                }
            };
            let (disposition, outputs, diagnostics, metadata) = execution.into_parts();
            if let Err(error) = validate::execution_metadata(profile, pass, step.mode(), &metadata)
            {
                push_failed_step(
                    &mut trace,
                    pass.id(),
                    step.mode(),
                    input_refs,
                    diagnostics,
                    error,
                    Some(metadata),
                )?;
                let trace = trace.fail()?;
                return Ok(DecisionRunReport::new(
                    DecisionRunOutcome::Failed,
                    trace,
                    artifacts,
                ));
            }

            match disposition {
                DecisionPassDisposition::Abstained => {
                    if !profile.exit().abstention_allowed() {
                        let error = DecisionError::AbstentionNotAllowed {
                            profile: profile.id(),
                        };
                        push_failed_step(
                            &mut trace,
                            pass.id(),
                            step.mode(),
                            input_refs,
                            diagnostics,
                            error,
                            Some(metadata),
                        )?;
                        let trace = trace.fail()?;
                        return Ok(DecisionRunReport::new(
                            DecisionRunOutcome::Failed,
                            trace,
                            artifacts,
                        ));
                    }
                    trace.push_step(DecisionTraceStep::recorded(
                        pass.id(),
                        step.mode(),
                        input_refs,
                        [],
                        diagnostics,
                        DecisionTraceStepStatus::Abstained,
                        DecisionVerifierResult::passed(),
                        Some(metadata),
                    ))?;
                    let trace = trace.abstain()?;
                    return Ok(DecisionRunReport::new(
                        DecisionRunOutcome::Abstained,
                        trace,
                        artifacts,
                    ));
                }
                DecisionPassDisposition::Completed => {
                    if let Err(error) = validate::executor_outputs(self.registry, pass, &outputs) {
                        push_failed_step(
                            &mut trace,
                            pass.id(),
                            step.mode(),
                            input_refs,
                            diagnostics,
                            error,
                            Some(metadata),
                        )?;
                        let trace = trace.fail()?;
                        return Ok(DecisionRunReport::new(
                            DecisionRunOutcome::Failed,
                            trace,
                            artifacts,
                        ));
                    }
                    let mut output_refs = Vec::with_capacity(outputs.len());
                    for output in outputs {
                        let (kind, role, payload, provenance) = output.into_parts();
                        let record =
                            trace.push_artifact(kind, role, Some(pass.id()), provenance)?;
                        let reference = record.artifact();
                        artifacts.insert(DecisionArtifact::new(record, payload))?;
                        flow.insert_artifact(role, kind, reference);
                        output_refs.push(reference);
                    }
                    trace.push_step(DecisionTraceStep::recorded(
                        pass.id(),
                        step.mode(),
                        input_refs,
                        output_refs,
                        diagnostics,
                        DecisionTraceStepStatus::Completed,
                        DecisionVerifierResult::passed(),
                        Some(metadata),
                    ))?;
                }
            }
        }

        if profile.exit().output().is_none() && profile.exit().abstention_allowed() {
            let trace = trace.abstain()?;
            return Ok(DecisionRunReport::new(
                DecisionRunOutcome::Abstained,
                trace,
                artifacts,
            ));
        }

        let terminal = flow.resolve_exit(profile)?;
        let trace = trace.complete()?;
        Ok(DecisionRunReport::new(
            DecisionRunOutcome::TerminalArtifact(terminal),
            trace,
            artifacts,
        ))
    }
}

fn push_failed_step(
    trace: &mut DecisionTraceBuilder,
    pass: world_core::DefinitionId,
    mode: ImplementationMode,
    input_refs: Vec<crate::DecisionInputRef>,
    diagnostics: impl IntoIterator<Item = DecisionPassDiagnostic>,
    error: DecisionError,
    metadata: Option<crate::DecisionExecutionMetadata>,
) -> Result<(), DecisionError> {
    let mut diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    diagnostics.push(DecisionPassDiagnostic::new(Some(pass), error.to_string())?);
    trace.push_step(DecisionTraceStep::recorded(
        pass,
        mode,
        input_refs,
        [],
        diagnostics,
        DecisionTraceStepStatus::Failed,
        DecisionVerifierResult::failed(),
        metadata,
    ))
}

fn allowed_context(
    profile: &crate::DecisionProfile,
    pass: &crate::DecisionPassContract,
) -> BTreeSet<ContextProjectionKind> {
    let profile_context = profile.context_inputs().collect::<BTreeSet<_>>();
    pass.allowed_context()
        .filter(|context| profile_context.contains(context))
        .collect()
}
