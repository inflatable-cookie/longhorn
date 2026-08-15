//! Property tests for the persisted linear-envelope decode path (card 213).
//! Two properties run 64 fixed cases each:
//!
//! - arbitrary bytes never panic `HistoryPersistence::load`, and loading is a
//!   pure function of the input bytes (two runs agree exactly);
//! - a valid envelope mutated by bit flips, truncation, and lies in numeric
//!   length or position fields fails classified (a typed `HistoryLoadError`)
//!   or loads, and a loaded authority re-encodes and re-loads cleanly.
//!
//! Measured cost: the whole module runs in well under one second.

use std::convert::Infallible;

use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryLimits, HistoryPayloadCodec,
    HistoryPayloadCodecFamily, HistoryPayloadCodecVersion, HistoryPersistence,
    HistoryPersistenceLimits, HistoryPolicy, LinearHistory,
};
use proptest::prelude::*;
use serde_json::Value;

use crate::support::*;

const CASES: u32 = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Bytes(Vec<u8>);

#[derive(Clone)]
struct BytesCodec {
    family: HistoryPayloadCodecFamily,
}

impl BytesCodec {
    fn new() -> Self {
        Self {
            family: HistoryPayloadCodecFamily::new("fixture.bytes").expect("fixture family"),
        }
    }
}

impl HistoryPayloadCodec<Bytes> for BytesCodec {
    type Error = Infallible;

    fn family(&self) -> &HistoryPayloadCodecFamily {
        &self.family
    }

    fn version(&self) -> HistoryPayloadCodecVersion {
        HistoryPayloadCodecVersion::new(1)
    }

    fn encode(&self, payload: &Bytes) -> Result<Vec<u8>, Self::Error> {
        Ok(payload.0.clone())
    }

    fn decode(&self, bytes: &[u8]) -> Result<Bytes, Self::Error> {
        Ok(Bytes(bytes.to_vec()))
    }
}

struct BytesPolicy;

impl HistoryPolicy<Bytes> for BytesPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &Bytes) -> Result<Bytes, Self::Error> {
        let mut reversed = payload.0.clone();
        reversed.reverse();
        Ok(Bytes(reversed))
    }

    fn is_noop(&self, payload: &Bytes) -> bool {
        payload.0.is_empty()
    }

    fn encoded_weight(&self, payload: &Bytes) -> Result<u64, Self::Error> {
        Ok(u64::try_from(payload.0.len()).unwrap_or(u64::MAX))
    }

    fn coalesce(
        &self,
        _: &Bytes,
        _: &Bytes,
        _: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<Bytes>, Self::Error> {
        Ok(HistoryCoalesce::KeepSeparate)
    }
}

fn persistence() -> HistoryPersistence<BytesCodec, longhorn_history::NoHistoryStructuralMigration> {
    HistoryPersistence::without_structural_migration(
        BytesCodec::new(),
        HistoryPersistenceLimits::new(64 * 1_024).expect("fixture persistence limits"),
    )
}

fn persisted_history() -> LinearHistory<Bytes> {
    let limits = HistoryLimits::new(10, 1_024, 64).expect("fixture limits");
    let mut history = LinearHistory::new(history_id("history:fuzz"), limits);
    for (index, body) in [
        (0_u64, vec![7_u8]),
        (1, vec![3_u8; 16]),
        (2, vec![9_u8; 64]),
    ] {
        history
            .record_applied(
                record(
                    index,
                    &format!("entry:fuzz-{index}"),
                    metadata(&format!("Fuzz entry {index}"), "fixture:bytes"),
                    Bytes(body),
                ),
                &BytesPolicy,
            )
            .expect("fixture record");
    }
    history
}

fn encoded() -> (LinearHistory<Bytes>, Vec<u8>) {
    let history = persisted_history();
    let bytes = persistence().encode(&history).expect("fixture encode");
    (history, bytes)
}

/// Writes a lying numeric value into one of the envelope's length, position,
/// or sequence fields — the fields a hand-edited or corrupt envelope uses to
/// disagree with its own payload bytes.
fn apply_numeric_lie(document: &mut Value, field: usize, value: u64) {
    let entry_index = usize::try_from(value).unwrap_or(0) % 3;
    match field % 6 {
        0 => document["currentPosition"] = Value::from(value),
        1 => document["nextSequence"] = Value::from(value),
        2 => document["limits"]["maximumEncodedWeight"] = Value::from(value),
        3 => document["limits"]["maximumEntries"] = Value::from(value),
        4 => document["entries"][entry_index]["encodedWeight"] = Value::from(value),
        _ => document["entries"][entry_index]["sequence"] = Value::from(value),
    }
}

/// ASCII- and JSON-shaped byte strings, occasionally prefixed with a valid
/// envelope fragment, so cases reach past JSON rejection into header checks.
fn hostile_bytes() -> impl Strategy<Value = Vec<u8>> {
    (
        prop::collection::vec(
            prop_oneof![
                3 => prop::sample::select(b"{}\":,[]0-9a-z".to_vec()),
                1 => any::<u8>(),
            ],
            0..=300,
        ),
        any::<bool>(),
    )
        .prop_map(|(bytes, prefix)| {
            if prefix {
                let (_, valid) = encoded();
                let take = bytes.len().min(valid.len());
                let mut prefixed = valid[..take].to_vec();
                prefixed.extend_from_slice(&bytes);
                prefixed
            } else {
                bytes
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// Arbitrary bytes never panic the loader, and every outcome is
    /// deterministic: two loads of the same bytes agree exactly.
    #[test]
    fn arbitrary_bytes_load_deterministically_without_panic(bytes in hostile_bytes()) {
        let history_id = history_id("history:fuzz");
        let first = persistence().load::<Bytes, BytesPolicy>(&history_id, &bytes, &BytesPolicy);
        let second = persistence().load::<Bytes, BytesPolicy>(&history_id, &bytes, &BytesPolicy);
        prop_assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }

    /// A valid envelope corrupted by numeric-field lies, bit flips, and
    /// truncation never panics the loader. When a mutated envelope still
    /// loads, the resulting authority re-encodes and re-loads cleanly — a
    /// load must never produce state the encoder cannot persist again.
    #[test]
    fn mutated_valid_envelopes_fail_classified_or_reencode(
        lies in prop::collection::vec((any::<usize>(), any::<u64>()), 0..=2),
        flips in prop::collection::vec((any::<usize>(), 0..8_u8), 0..=3),
        truncation in prop::option::of(any::<usize>()),
    ) {
        let (history, mut bytes) = encoded();
        for (field, value) in lies {
            if let Ok(mut document) = serde_json::from_slice::<Value>(&bytes) {
                apply_numeric_lie(&mut document, field, value);
                bytes = serde_json::to_vec(&document).expect("lie re-serialization");
            }
        }
        for (offset, bit) in flips {
            let offset = offset % bytes.len().max(1);
            if let Some(byte) = bytes.get_mut(offset) {
                *byte ^= 1 << bit;
            }
        }
        if let Some(len) = truncation {
            bytes.truncate(len % (bytes.len() + 1));
        }

        let first = persistence().load::<Bytes, BytesPolicy>(history.history_id(), &bytes, &BytesPolicy);
        let second = persistence().load::<Bytes, BytesPolicy>(history.history_id(), &bytes, &BytesPolicy);
        prop_assert_eq!(format!("{first:?}"), format!("{second:?}"));
        if let Ok(loaded) = first {
            let reencoded = persistence()
                .encode(loaded.history())
                .expect("a loaded authority must re-encode");
            persistence()
                .load::<Bytes, BytesPolicy>(history.history_id(), &reencoded, &BytesPolicy)
                .expect("a re-encoded authority must re-load");
        }
    }
}
