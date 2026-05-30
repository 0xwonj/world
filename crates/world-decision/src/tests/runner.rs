use world_context::ContextProjectionKind;

use crate::{
    DecisionArtifactPayload, DecisionError, DecisionExecutionMetadata, DecisionExecutorRegistry,
    DecisionInputRef, DecisionPassExecution, DecisionPassExecutionContext, DecisionPassExecutor,
    DecisionProfileExit, DecisionProfileOutput, DecisionRunOutcome, DecisionRunRequest,
    DecisionRunner, DecisionTraceStatus, DecisionTraceStepStatus, DecisionVerifierStatus,
    DeterminismPolicy, ImplementationMode, InputBinding, InputRequirement, ModelInvocationMetadata,
    OracleInvocationMetadata, PassClass, PassWritePolicy, ProducedDecisionArtifact,
    ProfileOraclePolicy, RepresentationInput, RepresentationOutput, RepresentationRole,
};
use world_core::DefinitionId;

use super::helpers::{
    context_projection, id, pass_with_metadata, profile_with_exit_and_policy,
    profile_with_terminal, representation, valid_two_step_registry,
};

#[derive(Clone)]
struct StaticExecutor {
    pass: DefinitionId,
    mode: ImplementationMode,
    determinism: DeterminismPolicy,
    output: Option<ProducedDecisionArtifact>,
    check_context: bool,
}

impl StaticExecutor {
    fn producing(
        pass: DefinitionId,
        mode: ImplementationMode,
        determinism: DeterminismPolicy,
        output: ProducedDecisionArtifact,
    ) -> Self {
        Self {
            pass,
            mode,
            determinism,
            output: Some(output),
            check_context: false,
        }
    }

    fn checking_context(pass: DefinitionId, output: ProducedDecisionArtifact) -> Self {
        Self {
            pass,
            mode: ImplementationMode::Rule,
            determinism: DeterminismPolicy::Deterministic,
            output: Some(output),
            check_context: true,
        }
    }
}

impl DecisionPassExecutor for StaticExecutor {
    fn pass_id(&self) -> DefinitionId {
        self.pass
    }

    fn mode(&self) -> ImplementationMode {
        self.mode
    }

    fn execute(
        &self,
        context: DecisionPassExecutionContext<'_>,
    ) -> Result<DecisionPassExecution, DecisionError> {
        if self.check_context {
            assert!(context.actor_context().observations().is_some());
            assert!(context.actor_context().social().is_none());
        }
        let metadata = DecisionExecutionMetadata::new(self.mode, self.determinism);
        let outputs = self.output.clone().into_iter();
        Ok(DecisionPassExecution::completed(outputs, metadata))
    }
}

fn marker(kind: u64, role: RepresentationRole) -> ProducedDecisionArtifact {
    ProducedDecisionArtifact::marker(id(kind), role)
}

fn assert_failed_report(report: &crate::DecisionRunReport, expected: &DecisionError) {
    assert_eq!(report.outcome(), DecisionRunOutcome::Failed);
    assert_eq!(report.trace().status(), DecisionTraceStatus::Failed);
    let Some(step) = report.trace().steps().last() else {
        panic!("failed report should include a failed step");
    };
    assert_eq!(step.status(), DecisionTraceStepStatus::Failed);
    assert_eq!(step.verifier().status(), DecisionVerifierStatus::Failed);
    assert!(
        step.diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.message() == expected.to_string()),
        "failed step should include diagnostic `{expected}`"
    );
}

#[derive(Clone, Debug)]
struct PayloadValue(u32);

#[test]
fn runner_executes_two_step_profile_and_records_trace() {
    let registry = valid_two_step_registry();
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(200),
            ImplementationMode::Rule,
            DeterminismPolicy::Deterministic,
            marker(100, RepresentationRole::DecisionSignal),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(201),
            ImplementationMode::Rule,
            DeterminismPolicy::Deterministic,
            marker(101, RepresentationRole::Choice),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));

    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(300), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert!(matches!(
        report.outcome(),
        DecisionRunOutcome::TerminalArtifact(_)
    ));
    assert_eq!(report.trace().status(), DecisionTraceStatus::Completed);
    assert_eq!(report.trace().steps().len(), 2);
    assert_eq!(
        report.trace().steps()[0].status(),
        DecisionTraceStepStatus::Completed
    );
    assert_eq!(
        report.trace().steps()[0].inputs(),
        [crate::DecisionInputRef::Context(
            ContextProjectionKind::Observation
        )]
    );
    assert_eq!(
        report.trace().steps()[1].inputs(),
        [crate::DecisionInputRef::Artifact(
            report.trace().steps()[0].outputs()[0]
        )]
    );
}

#[test]
fn runner_report_preserves_terminal_payload() {
    struct PayloadExecutor;

    impl DecisionPassExecutor for PayloadExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(20)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            _context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            let output = ProducedDecisionArtifact::new(
                id(10),
                RepresentationRole::Choice,
                DecisionArtifactPayload::new(PayloadValue(7)),
                world_context::ContextProvenance::new(),
            );
            Ok(DecisionPassExecution::completed(
                [output],
                DecisionExecutionMetadata::new(
                    ImplementationMode::Rule,
                    DeterminismPolicy::Deterministic,
                ),
            ))
        }
    }

    let choice = representation(10, "choice", [RepresentationRole::Choice]);
    let choose = pass_with_metadata(
        20,
        "choose",
        PassClass::Choice,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        30,
        "terminal_payload",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            choose.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::Choice,
        Some(choice.id()),
    );
    let registry = crate::DecisionRegistry::new([choice], [choose], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(PayloadExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));

    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    let payload = report
        .terminal_artifact()
        .and_then(|artifact| artifact.payload().downcast_ref::<PayloadValue>())
        .unwrap_or_else(|| panic!("terminal payload should be preserved"));
    assert_eq!(payload.0, 7);
}

#[test]
fn downstream_executor_reads_only_resolved_artifact_inputs() {
    struct ReadingExecutor;

    impl DecisionPassExecutor for ReadingExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(21)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            assert_eq!(context.inputs().len(), 1);
            assert!(matches!(
                context.inputs()[0].reference(),
                DecisionInputRef::Artifact(_)
            ));
            let payload = context
                .artifact(context.inputs()[0])
                .and_then(|artifact| artifact.payload().downcast_ref::<PayloadValue>())
                .unwrap_or_else(|| panic!("resolved artifact payload should be readable"));
            assert_eq!(payload.0, 41);

            Ok(DecisionPassExecution::completed(
                [marker(11, RepresentationRole::Choice)],
                DecisionExecutionMetadata::new(
                    ImplementationMode::Rule,
                    DeterminismPolicy::Deterministic,
                ),
            ))
        }
    }

    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let choice = representation(11, "choice", [RepresentationRole::Choice]);
    let ground = pass_with_metadata(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let choose = pass_with_metadata(
        21,
        "choose",
        PassClass::Choice,
        [RepresentationInput::required_kind(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice.id(),
        )],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        30,
        "scoped_artifact",
        [ContextProjectionKind::Observation],
        [
            crate::DecisionProfileStep::new(ground.id(), ImplementationMode::Rule),
            crate::DecisionProfileStep::new(choose.id(), ImplementationMode::Rule),
        ],
        RepresentationRole::Choice,
        Some(choice.id()),
    );
    let registry = crate::DecisionRegistry::new([signal, choice], [ground, choose], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(20),
            ImplementationMode::Rule,
            DeterminismPolicy::Deterministic,
            ProducedDecisionArtifact::new(
                id(10),
                RepresentationRole::DecisionSignal,
                DecisionArtifactPayload::new(PayloadValue(41)),
                world_context::ContextProvenance::new(),
            ),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    executors
        .add_executor(Box::new(ReadingExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));

    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert_eq!(report.trace().status(), DecisionTraceStatus::Completed);
}

#[test]
fn runner_returns_failed_trace_when_executor_is_missing() {
    let registry = valid_two_step_registry();
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(200),
            ImplementationMode::Rule,
            DeterminismPolicy::Deterministic,
            marker(100, RepresentationRole::DecisionSignal),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));

    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(300), &projection))
        .unwrap_or_else(|error| panic!("missing executor should be a failed report: {error}"));

    assert_eq!(report.outcome(), DecisionRunOutcome::Failed);
    assert_eq!(report.trace().status(), DecisionTraceStatus::Failed);
    assert_eq!(
        report.trace().steps()[1].status(),
        DecisionTraceStepStatus::Failed
    );
    assert_failed_report(
        &report,
        &DecisionError::MissingExecutor {
            profile: id(300),
            pass: id(201),
            mode: ImplementationMode::Rule,
        },
    );
}

#[test]
fn runner_restricts_context_access_to_pass_contract() {
    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        30,
        "context_restricted",
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Social,
        ],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::checking_context(
            id(20),
            marker(10, RepresentationRole::DecisionSignal),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert_eq!(report.trace().status(), DecisionTraceStatus::Completed);
}

#[test]
fn runner_restricts_context_access_to_profile_inputs() {
    let signal = representation(12, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        22,
        "ground_with_social_allowed",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Social,
        ],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        32,
        "profile_removes_social",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::checking_context(
            id(22),
            marker(12, RepresentationRole::DecisionSignal),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(32), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert_eq!(report.trace().status(), DecisionTraceStatus::Completed);
}

#[test]
fn runner_skips_disabled_step_without_executor_or_artifact() {
    let choice = representation(13, "choice", [RepresentationRole::Choice]);
    let disabled = pass_with_metadata(
        23,
        "disabled_diagnostic",
        PassClass::Validation,
        [],
        [],
        [],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Disabled],
        DeterminismPolicy::Deterministic,
    );
    let choose = pass_with_metadata(
        24,
        "choose",
        PassClass::Choice,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        34,
        "disabled_step",
        [ContextProjectionKind::Observation],
        [
            crate::DecisionProfileStep::new(disabled.id(), ImplementationMode::Disabled),
            crate::DecisionProfileStep::new(choose.id(), ImplementationMode::Rule),
        ],
        RepresentationRole::Choice,
        Some(choice.id()),
    );
    let registry = crate::DecisionRegistry::new([choice], [disabled, choose], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(24),
            ImplementationMode::Rule,
            DeterminismPolicy::Deterministic,
            marker(13, RepresentationRole::Choice),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));

    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(34), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert_eq!(
        report.trace().steps()[0].status(),
        DecisionTraceStepStatus::Skipped
    );
    assert!(report.trace().steps()[0].outputs().is_empty());
    assert_eq!(report.artifacts().len(), 1);
}

#[test]
fn runner_resolves_optional_absent_and_all_available_inputs() {
    struct InputShapeExecutor;

    impl DecisionPassExecutor for InputShapeExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(25)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            assert_eq!(context.inputs().len(), 2);
            assert!(context.inputs().iter().all(|input| {
                matches!(
                    input.reference(),
                    DecisionInputRef::Context(
                        ContextProjectionKind::Observation | ContextProjectionKind::Epistemic
                    )
                )
            }));

            Ok(DecisionPassExecution::completed(
                [marker(14, RepresentationRole::Choice)],
                DecisionExecutionMetadata::new(
                    ImplementationMode::Rule,
                    DeterminismPolicy::Deterministic,
                ),
            ))
        }
    }

    let choice = representation(14, "choice", [RepresentationRole::Choice]);
    let pass = pass_with_metadata(
        25,
        "input_shape",
        PassClass::Choice,
        [
            RepresentationInput::with_binding(
                RepresentationRole::ActorRelativeView,
                None,
                InputRequirement::Required,
                InputBinding::AllAvailable,
            )
            .unwrap_or_else(|error| panic!("input should build: {error}")),
            RepresentationInput::optional(RepresentationRole::SocialContextView),
        ],
        [RepresentationOutput::new(
            RepresentationRole::Choice,
            choice.id(),
        )],
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Epistemic,
        ],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        35,
        "optional_and_all",
        [
            ContextProjectionKind::Observation,
            ContextProjectionKind::Epistemic,
        ],
        [crate::DecisionProfileStep::new(
            pass.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::Choice,
        Some(choice.id()),
    );
    let registry = crate::DecisionRegistry::new([choice], [pass], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(InputShapeExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));

    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(35), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert_eq!(report.trace().status(), DecisionTraceStatus::Completed);
}

#[test]
fn runner_rejects_undeclared_executor_output() {
    let registry = valid_two_step_registry();
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(200),
            ImplementationMode::Rule,
            DeterminismPolicy::Deterministic,
            marker(101, RepresentationRole::Choice),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(300), &projection))
        .unwrap_or_else(|error| panic!("contract violation should return failed report: {error}"));
    assert_failed_report(
        &report,
        &DecisionError::UndeclaredExecutorOutput {
            pass: id(200),
            role: RepresentationRole::Choice,
            kind: id(101),
        },
    );
}

#[test]
fn runner_requires_model_metadata_for_llm_mode() {
    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "llm_ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Llm],
        DeterminismPolicy::ExternalNondeterministic,
    );
    let profile = profile_with_terminal(
        30,
        "llm",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Llm,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(StaticExecutor::producing(
            id(20),
            ImplementationMode::Llm,
            DeterminismPolicy::ExternalNondeterministic,
            marker(10, RepresentationRole::DecisionSignal),
        )))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("metadata violation should return failed report: {error}"));
    assert_failed_report(
        &report,
        &DecisionError::MissingModelMetadata { pass: id(20) },
    );
}

#[test]
fn runner_records_failed_trace_for_executor_error() {
    struct ErrorExecutor;

    impl DecisionPassExecutor for ErrorExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(20)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            _context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            Err(DecisionError::MissingExecutorPass { pass: id(99) })
        }
    }

    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        30,
        "executor_error",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(ErrorExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("executor error should return failed report: {error}"));
    assert_failed_report(
        &report,
        &DecisionError::MissingExecutorPass { pass: id(99) },
    );
}

#[test]
fn runner_records_failed_trace_for_disallowed_abstention() {
    struct AbstainingExecutor;

    impl DecisionPassExecutor for AbstainingExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(20)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            _context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            Ok(DecisionPassExecution::abstained(
                DecisionExecutionMetadata::new(
                    ImplementationMode::Rule,
                    DeterminismPolicy::Deterministic,
                ),
            ))
        }
    }

    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        30,
        "no_abstain",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(AbstainingExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| {
            panic!("abstention rejection should return failed report: {error}")
        });
    assert_failed_report(
        &report,
        &DecisionError::AbstentionNotAllowed { profile: id(30) },
    );
}

#[test]
fn runner_accepts_profile_allowed_abstention() {
    struct AbstainingExecutor;

    impl DecisionPassExecutor for AbstainingExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(20)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            _context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            Ok(DecisionPassExecution::abstained(
                DecisionExecutionMetadata::new(
                    ImplementationMode::Rule,
                    DeterminismPolicy::Deterministic,
                ),
            ))
        }
    }

    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_exit_and_policy(
        30,
        "may_abstain",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Rule,
        )],
        DecisionProfileExit::terminal_or_abstain(DecisionProfileOutput::kind(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )),
        ProfileOraclePolicy::Forbid,
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(AbstainingExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert_eq!(report.outcome(), DecisionRunOutcome::Abstained);
    assert_eq!(report.trace().status(), DecisionTraceStatus::Abstained);
    assert!(report.terminal_artifact().is_none());
}

#[test]
fn runner_rejects_oracle_metadata_in_normal_execution() {
    struct OracleContaminatedExecutor;

    impl DecisionPassExecutor for OracleContaminatedExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(20)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Rule
        }

        fn execute(
            &self,
            _context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            let oracle = OracleInvocationMetadata::new("ground_truth")?;
            let metadata = DecisionExecutionMetadata::new(
                ImplementationMode::Rule,
                DeterminismPolicy::Deterministic,
            )
            .with_oracle(oracle);
            Ok(DecisionPassExecution::completed(
                [marker(10, RepresentationRole::DecisionSignal)],
                metadata,
            ))
        }
    }

    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Rule],
        DeterminismPolicy::Deterministic,
    );
    let profile = profile_with_terminal(
        30,
        "normal",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Rule,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(OracleContaminatedExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);

    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| {
            panic!("oracle contamination should return failed report: {error}")
        });
    assert_failed_report(
        &report,
        &DecisionError::UnexpectedExecutionMetadata {
            pass: id(20),
            field: "oracle",
        },
    );
}

#[test]
fn runner_accepts_llm_mode_with_model_metadata() {
    struct LlmExecutor;

    impl DecisionPassExecutor for LlmExecutor {
        fn pass_id(&self) -> DefinitionId {
            id(20)
        }

        fn mode(&self) -> ImplementationMode {
            ImplementationMode::Llm
        }

        fn execute(
            &self,
            _context: DecisionPassExecutionContext<'_>,
        ) -> Result<DecisionPassExecution, DecisionError> {
            let model = ModelInvocationMetadata::new("model-a", "prompt-a", Some("temp=0.2"))?;
            let metadata = DecisionExecutionMetadata::new(
                ImplementationMode::Llm,
                DeterminismPolicy::ExternalNondeterministic,
            )
            .with_model(model);
            Ok(DecisionPassExecution::completed(
                [marker(10, RepresentationRole::DecisionSignal)],
                metadata,
            ))
        }
    }

    let signal = representation(10, "signal", [RepresentationRole::DecisionSignal]);
    let ground = pass_with_metadata(
        20,
        "llm_ground",
        PassClass::SemanticGrounding,
        [RepresentationInput::required(
            RepresentationRole::ObservationView,
        )],
        [RepresentationOutput::new(
            RepresentationRole::DecisionSignal,
            signal.id(),
        )],
        [ContextProjectionKind::Observation],
        [],
        [],
        PassWritePolicy::None,
        [ImplementationMode::Llm],
        DeterminismPolicy::ExternalNondeterministic,
    );
    let profile = profile_with_terminal(
        30,
        "llm",
        [ContextProjectionKind::Observation],
        [crate::DecisionProfileStep::new(
            ground.id(),
            ImplementationMode::Llm,
        )],
        RepresentationRole::DecisionSignal,
        Some(signal.id()),
    );
    let registry = crate::DecisionRegistry::new([signal], [ground], [profile])
        .unwrap_or_else(|error| panic!("registry should build: {error}"));
    let mut executors = DecisionExecutorRegistry::new();
    executors
        .add_executor(Box::new(LlmExecutor))
        .unwrap_or_else(|error| panic!("executor should register: {error}"));
    let runner = DecisionRunner::new(&registry, &executors)
        .unwrap_or_else(|error| panic!("runner should build: {error}"));
    let projection = context_projection(1);
    let report = runner
        .run(DecisionRunRequest::new(id(30), &projection))
        .unwrap_or_else(|error| panic!("profile should run: {error}"));

    assert!(report.trace().steps()[0].metadata().is_some());
}
