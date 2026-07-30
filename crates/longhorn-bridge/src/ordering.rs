use longhorn_core::{BridgeSessionId, DomainId};
use serde::{Deserialize, Serialize};

use crate::AuthorityEpoch;

/// Monotonic sequence within one domain authority epoch.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[cfg_attr(feature = "bindings", ts(type = "number"))]
#[serde(transparent)]
pub struct BridgeStreamSequence(u64);

impl BridgeStreamSequence {
    /// Initial sequence for an authoritative snapshot or stream.
    pub const INITIAL: Self = Self(0);

    /// Constructs a sequence from its serialized value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the serialized value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next sequence without wrapping.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

/// Session, domain, authority, and sequence evidence for one live projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeStreamCursor {
    session_id: BridgeSessionId,
    domain_id: DomainId,
    authority_epoch: AuthorityEpoch,
    sequence: BridgeStreamSequence,
}

impl BridgeStreamCursor {
    /// Constructs a stream cursor from authoritative evidence.
    #[must_use]
    pub const fn new(
        session_id: BridgeSessionId,
        domain_id: DomainId,
        authority_epoch: AuthorityEpoch,
        sequence: BridgeStreamSequence,
    ) -> Self {
        Self {
            session_id,
            domain_id,
            authority_epoch,
            sequence,
        }
    }

    /// Returns the authority session.
    #[must_use]
    pub const fn session_id(&self) -> &BridgeSessionId {
        &self.session_id
    }

    /// Returns the owning domain.
    #[must_use]
    pub const fn domain_id(&self) -> &DomainId {
        &self.domain_id
    }

    /// Returns the authority tenure.
    #[must_use]
    pub const fn authority_epoch(&self) -> AuthorityEpoch {
        self.authority_epoch
    }

    /// Returns monotonic position within the authority tenure.
    #[must_use]
    pub const fn sequence(&self) -> BridgeStreamSequence {
        self.sequence
    }
}

/// Authoritative current snapshot with consumer-owned payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeSnapshotEnvelope<P> {
    cursor: BridgeStreamCursor,
    payload: P,
}

impl<P> BridgeSnapshotEnvelope<P> {
    /// Wraps a consumer-owned authoritative snapshot.
    #[must_use]
    pub const fn new(cursor: BridgeStreamCursor, payload: P) -> Self {
        Self { cursor, payload }
    }

    /// Returns authoritative ordering evidence.
    #[must_use]
    pub const fn cursor(&self) -> &BridgeStreamCursor {
        &self.cursor
    }

    /// Returns the consumer-owned snapshot payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }
}

/// Authoritative live update with consumer-owned payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BridgeEventEnvelope<P> {
    cursor: BridgeStreamCursor,
    payload: P,
}

impl<P> BridgeEventEnvelope<P> {
    /// Wraps a consumer-owned authoritative update.
    #[must_use]
    pub const fn new(cursor: BridgeStreamCursor, payload: P) -> Self {
        Self { cursor, payload }
    }

    /// Returns authoritative ordering evidence.
    #[must_use]
    pub const fn cursor(&self) -> &BridgeStreamCursor {
        &self.cursor
    }

    /// Returns the consumer-owned event payload.
    #[must_use]
    pub const fn payload(&self) -> &P {
        &self.payload
    }
}

/// Result of presenting an authoritative snapshot to a stream tracker.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeSnapshotDecision {
    /// The snapshot becomes the current authoritative baseline.
    Accepted,
    /// The snapshot is valid but older than an event observed while unsynchronized.
    AcceptedResnapshotRequired,
    /// The snapshot belongs to a superseded or unknown session.
    SupersededSession,
    /// The snapshot belongs to another domain tracker.
    WrongDomain,
}

/// Deterministic classification of one live authoritative event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum BridgeStreamDecision {
    /// Apply the event and advance the accepted cursor.
    Apply,
    /// Ignore an event already represented by the current cursor.
    IgnoreDuplicate,
    /// Ignore an older event from the current authority tenure.
    IgnoreStale,
    /// Ignore an event from a superseded or unknown session.
    IgnoreSupersededSession,
    /// Ignore an event for another domain.
    IgnoreWrongDomain,
    /// Stop applying events and load a fresh snapshot because a sequence is missing.
    ResnapshotGap,
    /// Stop applying events and load a fresh snapshot for the changed authority tenure.
    ResnapshotNewEpoch,
    /// A prior gap, epoch change, or absent baseline still requires a snapshot.
    ResnapshotRequired,
}

/// Pure state machine for listener-first snapshot and live-event ordering.
#[derive(Clone, Debug)]
pub struct BridgeStreamTracker {
    current_session_id: BridgeSessionId,
    domain_id: DomainId,
    accepted_cursor: Option<BridgeStreamCursor>,
    pending_cursor: Option<BridgeStreamCursor>,
    requires_snapshot: bool,
}

impl BridgeStreamTracker {
    /// Constructs a tracker that requires its first authoritative snapshot.
    #[must_use]
    pub const fn new(current_session_id: BridgeSessionId, domain_id: DomainId) -> Self {
        Self {
            current_session_id,
            domain_id,
            accepted_cursor: None,
            pending_cursor: None,
            requires_snapshot: true,
        }
    }

    /// Advances to a negotiated session and requires a fresh snapshot.
    pub fn advance_session(&mut self, session_id: BridgeSessionId) {
        if self.current_session_id != session_id {
            self.current_session_id = session_id;
            self.accepted_cursor = None;
            self.pending_cursor = None;
            self.requires_snapshot = true;
        }
    }

    /// Accepts a current-session snapshot as the new authoritative baseline.
    pub fn accept_snapshot(&mut self, cursor: BridgeStreamCursor) -> BridgeSnapshotDecision {
        if cursor.session_id != self.current_session_id {
            return BridgeSnapshotDecision::SupersededSession;
        }
        if cursor.domain_id != self.domain_id {
            return BridgeSnapshotDecision::WrongDomain;
        }
        let pending_is_newer = self.pending_cursor.as_ref().is_some_and(|pending| {
            pending.authority_epoch > cursor.authority_epoch
                || (pending.authority_epoch == cursor.authority_epoch
                    && pending.sequence > cursor.sequence)
        });
        self.accepted_cursor = Some(cursor);
        self.pending_cursor = None;
        self.requires_snapshot = pending_is_newer;
        if pending_is_newer {
            BridgeSnapshotDecision::AcceptedResnapshotRequired
        } else {
            BridgeSnapshotDecision::Accepted
        }
    }

    /// Classifies and, only for a contiguous event, advances stream position.
    pub fn classify_event(&mut self, cursor: &BridgeStreamCursor) -> BridgeStreamDecision {
        if cursor.session_id != self.current_session_id {
            return BridgeStreamDecision::IgnoreSupersededSession;
        }
        if cursor.domain_id != self.domain_id {
            return BridgeStreamDecision::IgnoreWrongDomain;
        }
        if self.requires_snapshot {
            self.remember_pending(cursor);
            return BridgeStreamDecision::ResnapshotRequired;
        }
        let Some(current) = self.accepted_cursor.as_ref() else {
            self.requires_snapshot = true;
            self.remember_pending(cursor);
            return BridgeStreamDecision::ResnapshotRequired;
        };
        if cursor.authority_epoch < current.authority_epoch {
            return BridgeStreamDecision::IgnoreStale;
        }
        if cursor.authority_epoch > current.authority_epoch {
            self.requires_snapshot = true;
            self.remember_pending(cursor);
            return BridgeStreamDecision::ResnapshotNewEpoch;
        }
        if cursor.sequence == current.sequence {
            return BridgeStreamDecision::IgnoreDuplicate;
        }
        if cursor.sequence < current.sequence {
            return BridgeStreamDecision::IgnoreStale;
        }
        if current.sequence.checked_next() == Some(cursor.sequence) {
            self.accepted_cursor = Some(cursor.clone());
            return BridgeStreamDecision::Apply;
        }

        self.requires_snapshot = true;
        self.remember_pending(cursor);
        BridgeStreamDecision::ResnapshotGap
    }

    /// Returns the accepted authoritative cursor, if synchronized.
    #[must_use]
    pub const fn accepted_cursor(&self) -> Option<&BridgeStreamCursor> {
        self.accepted_cursor.as_ref()
    }

    /// Returns whether no further event may apply before a fresh snapshot.
    #[must_use]
    pub const fn requires_snapshot(&self) -> bool {
        self.requires_snapshot
    }

    fn remember_pending(&mut self, cursor: &BridgeStreamCursor) {
        let replace = self.pending_cursor.as_ref().is_none_or(|pending| {
            cursor.authority_epoch > pending.authority_epoch
                || (cursor.authority_epoch == pending.authority_epoch
                    && cursor.sequence > pending.sequence)
        });
        if replace {
            self.pending_cursor = Some(cursor.clone());
        }
    }
}
