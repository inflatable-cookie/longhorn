//! Placement staging and bounded flush port.
//!
//! Host-agnostic: a backend supplies a sink, and Longhorn stages
//! placements and waits on bounded flushes through it. Contract 020 names
//! this as "placement application", one of the capabilities any host must
//! provide.

use std::{
    sync::mpsc::{Receiver, channel},
    time::Duration,
};

use super::{CapturedWindowPlacement, WindowFlushRequest};

/// Sink-owned stage and bounded flush authority.
pub trait WindowPlacementSink: Send + Sync {
    /// Accepts one schema-opaque placement proposal.
    fn stage(&self, placement: &CapturedWindowPlacement) -> Result<(), String>;

    /// Starts one flush and returns its acknowledgement channel.
    fn request_flush(
        &self,
        request: &WindowFlushRequest,
    ) -> Result<WindowPlacementFlushTicket, String>;
}

/// Flush completion sent by an injected sink.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowPlacementFlushCompletion {
    /// Sink completed successfully.
    Succeeded,
    /// Sink completed with an inspectable failure.
    Failed(String),
}

/// One receiver used by the adapter to enforce its wait bound.
pub struct WindowPlacementFlushTicket {
    receiver: Receiver<WindowPlacementFlushCompletion>,
}

impl WindowPlacementFlushTicket {
    /// Wraps a sink completion receiver.
    #[must_use]
    pub const fn new(receiver: Receiver<WindowPlacementFlushCompletion>) -> Self {
        Self { receiver }
    }

    /// Constructs an already successful synchronous completion.
    #[must_use]
    pub fn completed() -> Self {
        Self::from_completion(WindowPlacementFlushCompletion::Succeeded)
    }

    /// Constructs an already failed synchronous completion.
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self::from_completion(WindowPlacementFlushCompletion::Failed(reason.into()))
    }

    fn from_completion(completion: WindowPlacementFlushCompletion) -> Self {
        let (sender, receiver) = channel();
        sender
            .send(completion)
            .expect("new completion receiver remains connected");
        Self::new(receiver)
    }

    /// Waits for the sink to acknowledge, bounded by the caller.
    pub fn wait(
        self,
        timeout_millis: u64,
    ) -> Result<WindowPlacementFlushCompletion, std::sync::mpsc::RecvTimeoutError> {
        self.receiver
            .recv_timeout(Duration::from_millis(timeout_millis))
    }
}
