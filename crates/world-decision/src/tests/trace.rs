use world_context::{ContextProvenance, ContextProvenanceSource, ContextReadSet};

use crate::{
    DecisionArtifactRecord, DecisionArtifactRef, DecisionError, DecisionPassDiagnostic,
    DecisionTrace, DecisionTraceHeader, DecisionTraceStatus, ImplementationMode,
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
    assert_eq!(step.inputs(), [artifact(1)]);
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
