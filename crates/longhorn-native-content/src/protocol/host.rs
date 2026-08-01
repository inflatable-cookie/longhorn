use longhorn_core::{
    NativeContentIslandId, NativeContentRequestId, NativeContentRevision, WindowId,
};

use crate::{
    ApplyPlan, ApplyReceipt, ContentSizeProposal, CoordinationError, HostDestroyReceipt,
    NativeContentCoordinator, ObservationReceipt, ObservationUpdate, ReceiptError, StepExecution,
};

use super::{
    NATIVE_CONTENT_PROTOCOL_VERSION, NativeContentAuthorityEpoch, NativeContentChangeProjection,
    NativeContentChangedEvent, NativeContentClientEpoch, NativeContentConnectRequest,
    NativeContentConnectResult, NativeContentContentSizeDecisionRequest,
    NativeContentContentSizeDecisionResult, NativeContentCursor, NativeContentDesiredUpdateRequest,
    NativeContentDesiredUpdateResult, NativeContentProtocolCounterError,
    NativeContentProtocolRejection, NativeContentProtocolVersion, NativeContentRejectionCode,
    NativeContentSnapshot, NativeContentSnapshotRequest, NativeContentSnapshotResult,
};

/// Pure island-scoped protocol authority shared by direct and native host adapters.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeContentProtocolHost {
    authority_epoch: NativeContentAuthorityEpoch,
    client_epoch: Option<NativeContentClientEpoch>,
    coordinator: NativeContentCoordinator,
}

impl NativeContentProtocolHost {
    /// Creates one protocol authority around an existing pure coordinator.
    #[must_use]
    pub const fn new(
        authority_epoch: NativeContentAuthorityEpoch,
        coordinator: NativeContentCoordinator,
    ) -> Self {
        Self {
            authority_epoch,
            client_epoch: None,
            coordinator,
        }
    }

    /// Returns the underlying pure authority for mechanism planning.
    #[must_use]
    pub const fn coordinator(&self) -> &NativeContentCoordinator {
        &self.coordinator
    }

    /// Returns the currently issued renderer epoch, when connected.
    #[must_use]
    pub const fn client_epoch(&self) -> Option<NativeContentClientEpoch> {
        self.client_epoch
    }

    /// Issues a fresh renderer epoch and returns current state.
    pub fn connect(&mut self, request: NativeContentConnectRequest) -> NativeContentConnectResult {
        let request_id = request.request_id;
        if let Err(rejection) = self.validate_envelope(request.protocol_version, &request.island_id)
        {
            return NativeContentConnectResult::Rejected {
                request_id,
                rejection,
            };
        }

        let next = match self.next_client_epoch() {
            Ok(epoch) => epoch,
            Err(error) => {
                return NativeContentConnectResult::Rejected {
                    request_id,
                    rejection: NativeContentProtocolRejection::admission(
                        NativeContentRejectionCode::ClientEpochExhausted,
                        error.to_string(),
                    ),
                };
            }
        };
        self.client_epoch = Some(next);
        NativeContentConnectResult::Connected {
            request_id,
            snapshot: Box::new(self.snapshot_for(next)),
        }
    }

    /// Returns current state only for the current renderer epoch.
    pub fn snapshot(&self, request: NativeContentSnapshotRequest) -> NativeContentSnapshotResult {
        let request_id = request.request_id;
        match self.validate_session(
            request.protocol_version,
            &request.island_id,
            request.client_epoch,
        ) {
            Ok(()) => NativeContentSnapshotResult::Ready {
                request_id,
                snapshot: Box::new(self.snapshot_for(request.client_epoch)),
            },
            Err(rejection) => NativeContentSnapshotResult::Rejected {
                request_id,
                rejection,
            },
        }
    }

    /// Replaces desired state under current renderer and revision authority.
    pub fn update_desired(
        &mut self,
        request: NativeContentDesiredUpdateRequest,
    ) -> NativeContentDesiredUpdateResult {
        let request_id = request.request_id;
        if let Err(rejection) = self.validate_session(
            request.protocol_version,
            &request.island_id,
            request.client_epoch,
        ) {
            return NativeContentDesiredUpdateResult::Rejected {
                request_id,
                rejection,
            };
        }

        match self
            .coordinator
            .update_desired(request.expected_desired_revision, request.update)
        {
            Ok(receipt) => {
                let event = self.event(NativeContentChangeProjection::DesiredUpdated {
                    request_id: request_id.clone(),
                    receipt,
                });
                NativeContentDesiredUpdateResult::Committed {
                    request_id,
                    snapshot: Box::new(self.snapshot_for(request.client_epoch)),
                    receipt,
                    event: Box::new(event),
                }
            }
            Err(error) => NativeContentDesiredUpdateResult::Rejected {
                request_id,
                rejection: error.into(),
            },
        }
    }

    /// Records a consumer decision for one current size proposal.
    pub fn decide_content_size(
        &self,
        request: NativeContentContentSizeDecisionRequest,
    ) -> NativeContentContentSizeDecisionResult {
        let request_id = request.request_id;
        if let Err(rejection) = self.validate_session(
            request.protocol_version,
            &request.island_id,
            request.client_epoch,
        ) {
            return NativeContentContentSizeDecisionResult::Rejected {
                request_id,
                rejection,
            };
        }

        match self
            .coordinator
            .decide_content_size(request.proposal, request.decision)
        {
            Ok(receipt) => {
                let event = self.event(NativeContentChangeProjection::ContentSizeDecided {
                    request_id: request_id.clone(),
                    receipt: receipt.clone(),
                });
                NativeContentContentSizeDecisionResult::Decided {
                    request_id,
                    snapshot: Box::new(self.snapshot_for(request.client_epoch)),
                    receipt,
                    event: Box::new(event),
                }
            }
            Err(error) => NativeContentContentSizeDecisionResult::Rejected {
                request_id,
                rejection: error.into(),
            },
        }
    }

    /// Admits trusted adapter observation and projects it to the current renderer.
    pub fn admit_observation(
        &mut self,
        request_id: Option<NativeContentRequestId>,
        expected_observed_revision: NativeContentRevision,
        update: ObservationUpdate,
    ) -> Result<(ObservationReceipt, Option<NativeContentChangedEvent>), CoordinationError> {
        let receipt = self
            .coordinator
            .admit_observation(expected_observed_revision, update)?;
        let event = self.current_event(NativeContentChangeProjection::ObservationAdmitted {
            request_id,
            receipt,
        });
        Ok((receipt, event))
    }

    /// Validates and projects one mechanism-originated size proposal.
    pub fn publish_content_size_proposal(
        &self,
        request_id: NativeContentRequestId,
        proposal: ContentSizeProposal,
    ) -> Result<Option<NativeContentChangedEvent>, CoordinationError> {
        self.coordinator.validate_content_size_proposal(proposal)?;
        Ok(
            self.current_event(NativeContentChangeProjection::ContentSizeProposed {
                request_id,
                proposal,
            }),
        )
    }

    /// Builds and projects exact adapter completion evidence for one current plan.
    pub fn complete_apply(
        &self,
        request_id: NativeContentRequestId,
        plan: &ApplyPlan,
        executions: impl IntoIterator<Item = StepExecution>,
    ) -> Result<(ApplyReceipt, Option<NativeContentChangedEvent>), ReceiptError> {
        let receipt = self.coordinator.receipt(plan, executions)?;
        let event = self.current_event(NativeContentChangeProjection::ApplyCompleted {
            request_id,
            receipt: receipt.clone(),
        });
        Ok((receipt, event))
    }

    /// Invalidates a destroyed host before later native events can enter.
    pub fn host_destroyed(
        &mut self,
        request_id: Option<NativeContentRequestId>,
        host_window_id: &WindowId,
        expected_observed_revision: NativeContentRevision,
    ) -> Result<(HostDestroyReceipt, Option<NativeContentChangedEvent>), CoordinationError> {
        let receipt = self
            .coordinator
            .host_destroyed(host_window_id, expected_observed_revision)?;
        let event = self.current_event(NativeContentChangeProjection::HostDestroyed {
            request_id,
            receipt,
        });
        Ok((receipt, event))
    }

    fn next_client_epoch(
        &self,
    ) -> Result<NativeContentClientEpoch, NativeContentProtocolCounterError> {
        match self.client_epoch {
            Some(epoch) => epoch.checked_next(),
            None => NativeContentClientEpoch::new(1),
        }
    }

    fn validate_session(
        &self,
        version: NativeContentProtocolVersion,
        island_id: &NativeContentIslandId,
        client_epoch: NativeContentClientEpoch,
    ) -> Result<(), NativeContentProtocolRejection> {
        self.validate_envelope(version, island_id)?;
        let current = self.client_epoch.ok_or_else(|| {
            NativeContentProtocolRejection::admission(
                NativeContentRejectionCode::FutureClientEpoch,
                "no renderer session has been issued",
            )
        })?;
        if client_epoch < current {
            Err(NativeContentProtocolRejection::admission(
                NativeContentRejectionCode::StaleClientEpoch,
                format!(
                    "renderer epoch {} is stale; current is {}",
                    client_epoch.get(),
                    current.get()
                ),
            ))
        } else if client_epoch > current {
            Err(NativeContentProtocolRejection::admission(
                NativeContentRejectionCode::FutureClientEpoch,
                format!(
                    "renderer epoch {} was not issued; current is {}",
                    client_epoch.get(),
                    current.get()
                ),
            ))
        } else {
            Ok(())
        }
    }

    fn validate_envelope(
        &self,
        version: NativeContentProtocolVersion,
        island_id: &NativeContentIslandId,
    ) -> Result<(), NativeContentProtocolRejection> {
        if version != NativeContentProtocolVersion::CURRENT {
            return Err(NativeContentProtocolRejection::compatibility(format!(
                "native-content protocol {} is unsupported; expected {NATIVE_CONTENT_PROTOCOL_VERSION}",
                version.get()
            )));
        }
        if island_id != self.coordinator.desired().island_id() {
            return Err(NativeContentProtocolRejection::admission(
                NativeContentRejectionCode::IslandMismatch,
                format!(
                    "island {island_id} does not match {}",
                    self.coordinator.desired().island_id()
                ),
            ));
        }
        Ok(())
    }

    fn snapshot_for(&self, client_epoch: NativeContentClientEpoch) -> NativeContentSnapshot {
        NativeContentSnapshot {
            protocol_version: NativeContentProtocolVersion::CURRENT,
            cursor: self.cursor(client_epoch),
            desired: self.coordinator.desired().clone(),
            observed: self.coordinator.observed().clone(),
            invalidated_generation: self.coordinator.invalidated_generation(),
        }
    }

    fn cursor(&self, client_epoch: NativeContentClientEpoch) -> NativeContentCursor {
        NativeContentCursor {
            authority_epoch: self.authority_epoch,
            client_epoch,
            island_id: self.coordinator.desired().island_id().clone(),
            attach_generation: self.coordinator.desired().generation(),
            desired_revision: self.coordinator.desired().revision(),
            observed_revision: self.coordinator.observed().revision(),
        }
    }

    fn current_event(
        &self,
        change: NativeContentChangeProjection,
    ) -> Option<NativeContentChangedEvent> {
        self.client_epoch.map(|_| self.event(change))
    }

    fn event(&self, change: NativeContentChangeProjection) -> NativeContentChangedEvent {
        let client_epoch = self
            .client_epoch
            .expect("native-content event requires an issued client epoch");
        NativeContentChangedEvent {
            protocol_version: NativeContentProtocolVersion::CURRENT,
            cursor: self.cursor(client_epoch),
            change,
        }
    }
}
