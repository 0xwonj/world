use world_context::{ContextProvenance, ContextProvenanceSource, ContextReadSet};

use crate::{
    DecisionArtifactRecord, DecisionArtifactRef, DecisionError, DecisionInputRef,
    DecisionPassDiagnostic, DecisionTrace, DecisionTraceBuilder, DecisionTraceHeader,
    DecisionTraceStatus, DecisionTraceStepStatus, DecisionVerifierResult, ImplementationMode,
    ProfileOraclePolicy, RepresentationRole,
};

use super::helpers::{actor, id, version};

fn artifact(value: u64) -> DecisionArtifactRef {
    let Some(artifact) = DecisionArtifactRef::new(value) else {
        panic!("test artifact refs must be nonzero");
    };
    artifact
}

fn header(policy: ProfileOraclePolicy) -> DecisionTraceHeader {
    DecisionTraceHeader::new(
        actor(1),
        id(10),
        version(1),
        ContextReadSet::new(),
        ContextProvenance::new(),
        policy,
    )
}

#[test]
fn artifact_refs_are_trace_local_and_nonzero() {
    assert_eq!(DecisionArtifactRef::new(0), None);
    assert_eq!(artifact(1).get(), 1);
    assert!(artifact(1) < artifact(2));
}

#[test]
fn trace_header_carries_context_reads_and_provenance() {
    let mut reads = ContextReadSet::new();
    assert!(reads.insert_definition(id(99)));
    let mut provenance = ContextProvenance::new();
    provenance.push(ContextProvenanceSource::Definition(id(99)));

    let header = DecisionTraceHeader::new(
        actor(7),
        id(10),
        version(3),
        reads,
        provenance,
        ProfileOraclePolicy::Forbid,
    );

    assert_eq!(header.actor(), actor(7));
    assert!(header.context_reads().contains_definition(id(99)));
    assert_eq!(
        header.context_provenance().sources(),
        [ContextProvenanceSource::Definition(id(99))]
    );
}

#[test]
fn trace_rejects_duplicate_artifact_refs() {
    let record = DecisionArtifactRecord::new(
        artifact(1),
        id(100),
        RepresentationRole::DecisionSignal,
        Some(id(20)),
        ContextProvenance::new(),
    );

    assert_eq!(
        DecisionTrace::from_parts(
            header(ProfileOraclePolicy::Forbid),
            [],
            [record.clone(), record],
            DecisionTraceStatus::Completed,
        ),
        Err(DecisionError::DuplicateArtifactRef {
            artifact: artifact(1),
        })
    );
}

#[test]
fn trace_rejects_missing_artifact_input_in_from_parts() {
    let step = crate::trace::DecisionTraceStep::recorded(
        id(20),
        ImplementationMode::Rule,
        [DecisionInputRef::Artifact(artifact(7))],
        [],
        [],
        DecisionTraceStepStatus::Completed,
        DecisionVerifierResult::passed(),
        None,
    );

    assert_eq!(
        DecisionTrace::from_parts(
            header(ProfileOraclePolicy::Forbid),
            [step],
            [],
            DecisionTraceStatus::Completed,
        ),
        Err(DecisionError::MissingTraceArtifact {
            artifact: artifact(7),
        })
    );
}

#[test]
fn trace_rejects_output_producer_mismatch_in_from_parts() {
    let record = DecisionArtifactRecord::new(
        artifact(1),
        id(100),
        RepresentationRole::DecisionSignal,
        Some(id(21)),
        ContextProvenance::new(),
    );
    let step = crate::trace::DecisionTraceStep::recorded(
        id(20),
        ImplementationMode::Rule,
        [],
        [artifact(1)],
        [],
        DecisionTraceStepStatus::Completed,
        DecisionVerifierResult::passed(),
        None,
    );

    assert_eq!(
        DecisionTrace::from_parts(
            header(ProfileOraclePolicy::Forbid),
            [step],
            [record],
            DecisionTraceStatus::Completed,
        ),
        Err(DecisionError::TraceOutputProducerMismatch {
            pass: id(20),
            artifact: artifact(1),
            producer: Some(id(21)),
        })
    );
}

#[test]
fn trace_records_explicit_oracle_policy() {
    let trace = DecisionTrace::new(header(ProfileOraclePolicy::Allow));

    assert_eq!(trace.header().oracle_policy(), ProfileOraclePolicy::Allow);
    assert_eq!(trace.status(), DecisionTraceStatus::Started);
}

#[test]
fn trace_step_and_diagnostic_are_value_like() {
    let Ok(diagnostic) = DecisionPassDiagnostic::new(Some(id(20)), "candidate pruned") else {
        panic!("test diagnostic should be valid");
    };
    let step = crate::trace::DecisionTraceStep::new(
        id(20),
        ImplementationMode::Rule,
        [artifact(1)],
        [artifact(2)],
        [diagnostic],
    );

    assert_eq!(step.pass(), id(20));
    assert_eq!(step.inputs(), [DecisionInputRef::Artifact(artifact(1))]);
    assert_eq!(step.outputs(), [artifact(2)]);
    assert_eq!(step.diagnostics()[0].message(), "candidate pruned");
}

#[test]
fn diagnostic_rejects_empty_message() {
    assert_eq!(
        DecisionPassDiagnostic::new(Some(id(20)), " "),
        Err(DecisionError::EmptyItemField {
            type_name: "DecisionPassDiagnostic",
            field: "message",
        })
    );
}

#[test]
fn trace_step_records_context_inputs_and_status() {
    let step = crate::trace::DecisionTraceStep::recorded(
        id(20),
        ImplementationMode::Rule,
        [DecisionInputRef::Context(
            world_context::ContextProjectionKind::Observation,
        )],
        [],
        [],
        DecisionTraceStepStatus::Skipped,
        DecisionVerifierResult::not_run(),
        None,
    );

    assert_eq!(
        step.inputs(),
        [DecisionInputRef::Context(
            world_context::ContextProjectionKind::Observation
        )]
    );
    assert_eq!(step.status(), DecisionTraceStepStatus::Skipped);
}

#[test]
fn trace_builder_rejects_missing_artifact_input() {
    let mut builder = DecisionTraceBuilder::new(header(ProfileOraclePolicy::Forbid));
    let step = crate::trace::DecisionTraceStep::recorded(
        id(20),
        ImplementationMode::Rule,
        [DecisionInputRef::Artifact(artifact(7))],
        [],
        [],
        DecisionTraceStepStatus::Completed,
        DecisionVerifierResult::passed(),
        None,
    );

    assert_eq!(
        builder.push_step(step),
        Err(DecisionError::MissingTraceArtifact {
            artifact: artifact(7),
        })
    );
}

#[test]
fn trace_builder_records_completed_trace() {
    let mut builder = DecisionTraceBuilder::new(header(ProfileOraclePolicy::Forbid));
    let record = builder
        .push_artifact(
            id(100),
            RepresentationRole::DecisionSignal,
            Some(id(20)),
            ContextProvenance::new(),
        )
        .unwrap_or_else(|error| panic!("artifact should record: {error}"));
    builder
        .push_step(crate::trace::DecisionTraceStep::recorded(
            id(20),
            ImplementationMode::Rule,
            [],
            [record.artifact()],
            [],
            DecisionTraceStepStatus::Completed,
            DecisionVerifierResult::passed(),
            None,
        ))
        .unwrap_or_else(|error| panic!("step should record: {error}"));

    let trace = builder
        .complete()
        .unwrap_or_else(|error| panic!("trace should complete: {error}"));

    assert_eq!(trace.status(), DecisionTraceStatus::Completed);
    assert_eq!(trace.steps().len(), 1);
    assert_eq!(trace.artifacts().len(), 1);
}
