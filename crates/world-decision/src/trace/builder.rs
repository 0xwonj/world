use std::collections::BTreeMap;

use world_context::ContextProvenance;
use world_core::DefinitionId;

use crate::{
    DecisionArtifactRecord, DecisionArtifactRef, DecisionError, DecisionInputRef, DecisionTrace,
    DecisionTraceHeader, DecisionTraceStatus, DecisionTraceStep, RepresentationRole,
};

/// Builder for trace records produced by the decision runner.
#[derive(Clone, Debug)]
pub struct DecisionTraceBuilder {
    header: DecisionTraceHeader,
    steps: Vec<DecisionTraceStep>,
    artifacts: BTreeMap<DecisionArtifactRef, DecisionArtifactRecord>,
    next_artifact: u64,
}

impl DecisionTraceBuilder {
    /// Creates an empty trace builder.
    #[must_use]
    pub fn new(header: DecisionTraceHeader) -> Self {
        Self {
            header,
            steps: Vec::new(),
            artifacts: BTreeMap::new(),
            next_artifact: 1,
        }
    }

    /// Allocates and records a new artifact.
    pub fn push_artifact(
        &mut self,
        kind: DefinitionId,
        role: RepresentationRole,
        producer: Option<DefinitionId>,
        provenance: ContextProvenance,
    ) -> Result<DecisionArtifactRecord, DecisionError> {
        let artifact = self.allocate_artifact_ref();
        let record = DecisionArtifactRecord::new(artifact, kind, role, producer, provenance);
        self.insert_artifact(record.clone())?;
        Ok(record)
    }

    /// Inserts an already allocated artifact record.
    pub fn insert_artifact(&mut self, record: DecisionArtifactRecord) -> Result<(), DecisionError> {
        if self.artifacts.contains_key(&record.artifact()) {
            return Err(DecisionError::DuplicateArtifactRef {
                artifact: record.artifact(),
            });
        }
        self.artifacts.insert(record.artifact(), record);
        Ok(())
    }

    /// Records one trace step after validating artifact references.
    pub fn push_step(&mut self, step: DecisionTraceStep) -> Result<(), DecisionError> {
        self.validate_inputs(step.inputs())?;
        self.validate_outputs(step.pass(), step.outputs())?;
        self.steps.push(step);
        Ok(())
    }

    /// Finalizes the trace as completed.
    pub fn complete(self) -> Result<DecisionTrace, DecisionError> {
        self.finish(DecisionTraceStatus::Completed)
    }

    /// Finalizes the trace as abstained.
    pub fn abstain(self) -> Result<DecisionTrace, DecisionError> {
        self.finish(DecisionTraceStatus::Abstained)
    }

    /// Finalizes the trace as failed.
    pub fn fail(self) -> Result<DecisionTrace, DecisionError> {
        self.finish(DecisionTraceStatus::Failed)
    }

    fn allocate_artifact_ref(&mut self) -> DecisionArtifactRef {
        loop {
            let candidate = self.next_artifact;
            self.next_artifact = self.next_artifact.saturating_add(1);
            if let Some(artifact) = DecisionArtifactRef::new(candidate)
                && !self.artifacts.contains_key(&artifact)
            {
                return artifact;
            }
        }
    }

    fn validate_inputs(&self, inputs: &[DecisionInputRef]) -> Result<(), DecisionError> {
        for input in inputs {
            if let DecisionInputRef::Artifact(artifact) = input
                && !self.artifacts.contains_key(artifact)
            {
                return Err(DecisionError::MissingTraceArtifact {
                    artifact: *artifact,
                });
            }
        }
        Ok(())
    }

    fn validate_outputs(
        &self,
        pass: DefinitionId,
        outputs: &[DecisionArtifactRef],
    ) -> Result<(), DecisionError> {
        for output in outputs {
            let Some(record) = self.artifacts.get(output) else {
                return Err(DecisionError::MissingTraceArtifact { artifact: *output });
            };
            if record.producer() != Some(pass) {
                return Err(DecisionError::TraceOutputProducerMismatch {
                    pass,
                    artifact: *output,
                    producer: record.producer(),
                });
            }
        }
        Ok(())
    }

    fn finish(self, status: DecisionTraceStatus) -> Result<DecisionTrace, DecisionError> {
        DecisionTrace::from_parts(
            self.header,
            self.steps,
            self.artifacts.into_values(),
            status,
        )
    }
}
