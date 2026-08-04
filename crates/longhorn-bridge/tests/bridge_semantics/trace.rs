use longhorn_bridge::{
    AuthorityEpoch, AuthorityRevision, BridgeCommandDelivery, BridgeCommandEnvelope,
    BridgeCommandRetryDecision, BridgeDeduplicationCapacity, BridgeDeduplicationLedger,
    BridgeRetryClass,
};
use longhorn_core::BridgeIdempotencyKey;

use crate::support::{CommandPayload, context, idempotency_key, request_id};

#[derive(Debug)]
struct FixtureAuthority {
    value: i64,
    revision: u64,
    epoch: AuthorityEpoch,
    deduplication: BridgeDeduplicationLedger<i64>,
}

impl FixtureAuthority {
    fn apply(&mut self, command: &BridgeCommandEnvelope<CommandPayload>) -> bool {
        if command.authority_epoch() != self.epoch
            || command.expected_revision().map(AuthorityRevision::get) != Some(self.revision)
        {
            return false;
        }
        if let Some(key) = command.idempotency_key()
            && self.deduplication.lookup(key).is_some()
        {
            return false;
        }

        let candidate = self.value.checked_add(command.payload().delta);
        let Some(candidate) = candidate else {
            return false;
        };
        if let Some(key) = command.idempotency_key()
            && self
                .deduplication
                .record(
                    BridgeIdempotencyKey::new(key.as_str()).unwrap(),
                    request_id(command.context().request_id().as_str()),
                    candidate,
                )
                .is_err()
        {
            return false;
        }
        self.value = candidate;
        self.revision += 1;
        true
    }
}

#[test]
fn invalid_and_uncertain_trace_steps_leave_fixture_state_unchanged() {
    let mut authority = FixtureAuthority {
        value: 10,
        revision: 4,
        epoch: AuthorityEpoch::new(2).unwrap(),
        deduplication: BridgeDeduplicationLedger::new(BridgeDeduplicationCapacity::new(2).unwrap()),
    };
    let stale = BridgeCommandEnvelope::new(
        context("request:stale"),
        AuthorityEpoch::new(2).unwrap(),
        Some(AuthorityRevision::new(3)),
        Some(idempotency_key("idempotency:stale")),
        CommandPayload { delta: 5 },
    );
    assert!(!authority.apply(&stale));
    assert_eq!((authority.value, authority.revision), (10, 4));

    let uncertain_non_idempotent = BridgeCommandEnvelope::new(
        context("request:uncertain"),
        AuthorityEpoch::new(2).unwrap(),
        Some(AuthorityRevision::new(4)),
        None,
        CommandPayload { delta: 5 },
    );
    assert_eq!(
        uncertain_non_idempotent.classify_transport_failure(
            BridgeCommandDelivery::Uncertain,
            BridgeRetryClass::AfterReconnect,
            authority.deduplication.support(),
        ),
        BridgeCommandRetryDecision::Indeterminate
    );
    assert_eq!((authority.value, authority.revision), (10, 4));

    let valid = BridgeCommandEnvelope::new(
        context("request:valid"),
        AuthorityEpoch::new(2).unwrap(),
        Some(AuthorityRevision::new(4)),
        Some(idempotency_key("idempotency:valid")),
        CommandPayload { delta: 5 },
    );
    assert!(authority.apply(&valid));
    assert_eq!((authority.value, authority.revision), (15, 5));
    assert!(!authority.apply(&valid));
    assert_eq!((authority.value, authority.revision), (15, 5));
}
