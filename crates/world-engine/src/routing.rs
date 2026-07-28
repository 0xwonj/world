use world_runtime::{
    EvidenceObservation, MomentWorkDecision, MomentWorkInput, PostCommitRoutingDecision,
    PostCommitRoutingPolicyV1, RuntimeEvaluationError,
};

/// Pure engine-owned interpretation of post-commit work for one sealed execution.
pub(crate) struct PostCommitRouter {
    policy: PostCommitRoutingPolicyV1,
}

impl PostCommitRouter {
    pub(crate) const fn new(policy: PostCommitRoutingPolicyV1) -> Self {
        Self { policy }
    }

    pub(crate) fn route(
        &self,
        input: MomentWorkInput<'_>,
    ) -> Result<MomentWorkDecision, RuntimeEvaluationError> {
        match self.policy {
            PostCommitRoutingPolicyV1::DirectActorEvidence => {
                let MomentWorkInput::PostCommitDispatch { dispatch, .. } = input else {
                    return Err(RuntimeEvaluationError::Integrity);
                };
                let observations = dispatch
                    .reaction()
                    .events()
                    .iter()
                    .enumerate()
                    .map(|(event_index, event)| {
                        let event_index = u32::try_from(event_index)
                            .map_err(|_| RuntimeEvaluationError::Integrity)?;
                        Ok(EvidenceObservation::direct(event.actor(), event_index))
                    })
                    .collect::<Result<Vec<_>, RuntimeEvaluationError>>()?;
                MomentWorkDecision::route_post_commit(
                    input,
                    PostCommitRoutingDecision::DeliverEvidence(observations),
                )
                .map_err(|_| RuntimeEvaluationError::Integrity)
            }
        }
    }
}
