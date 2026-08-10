use std::{error::Error, fmt};

use longhorn_core::HistoryRevision;
use longhorn_history::{
    HistoryCoalesce, HistoryCoalesceContext, HistoryEntrySequence, HistoryLimits, HistoryPolicy,
    HistoryRecordOutcome, LinearHistory, LinearHistoryState,
};
use serde::{Deserialize, Serialize};

use crate::support::*;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) enum PulseFixtureMutation {
    RenameTrack {
        track_id: u32,
        before: String,
        after: String,
    },
    DeleteTrack {
        track_id: u32,
        snapshot: String,
    },
    RestoreTrack {
        track_id: u32,
        snapshot: String,
    },
    Compound {
        mutations: Vec<PulseFixtureMutation>,
    },
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PulseFixturePolicyError;

impl fmt::Display for PulseFixturePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("fixture mutation is not invertible")
    }
}

impl Error for PulseFixturePolicyError {}

pub(crate) struct PulseFixturePolicy;

impl HistoryPolicy<PulseFixtureMutation> for PulseFixturePolicy {
    type Error = PulseFixturePolicyError;

    fn inverse(&self, payload: &PulseFixtureMutation) -> Result<PulseFixtureMutation, Self::Error> {
        match payload {
            PulseFixtureMutation::RenameTrack {
                track_id,
                before,
                after,
            } => Ok(PulseFixtureMutation::RenameTrack {
                track_id: *track_id,
                before: after.clone(),
                after: before.clone(),
            }),
            PulseFixtureMutation::DeleteTrack { track_id, snapshot } => {
                Ok(PulseFixtureMutation::RestoreTrack {
                    track_id: *track_id,
                    snapshot: snapshot.clone(),
                })
            }
            PulseFixtureMutation::RestoreTrack { track_id, snapshot } => {
                Ok(PulseFixtureMutation::DeleteTrack {
                    track_id: *track_id,
                    snapshot: snapshot.clone(),
                })
            }
            PulseFixtureMutation::Compound { mutations } => Ok(PulseFixtureMutation::Compound {
                mutations: mutations
                    .iter()
                    .rev()
                    .map(|mutation| self.inverse(mutation))
                    .collect::<Result<Vec<_>, _>>()?,
            }),
            PulseFixtureMutation::Unsupported => Err(PulseFixturePolicyError),
        }
    }

    fn is_noop(&self, payload: &PulseFixtureMutation) -> bool {
        matches!(
            payload,
            PulseFixtureMutation::RenameTrack { before, after, .. } if before == after
        ) || matches!(
            payload,
            PulseFixtureMutation::Compound { mutations }
                if mutations.iter().all(|mutation| self.is_noop(mutation))
        )
    }

    fn encoded_weight(&self, payload: &PulseFixtureMutation) -> Result<u64, Self::Error> {
        match payload {
            PulseFixtureMutation::RenameTrack { before, after, .. } => {
                u64::try_from(before.len() + after.len() + std::mem::size_of::<u32>())
                    .map_err(|_| PulseFixturePolicyError)
            }
            PulseFixtureMutation::DeleteTrack { snapshot, .. }
            | PulseFixtureMutation::RestoreTrack { snapshot, .. } => {
                u64::try_from(snapshot.len() + std::mem::size_of::<u32>())
                    .map_err(|_| PulseFixturePolicyError)
            }
            PulseFixtureMutation::Compound { mutations } => {
                mutations.iter().try_fold(0_u64, |total, mutation| {
                    total
                        .checked_add(self.encoded_weight(mutation)?)
                        .ok_or(PulseFixturePolicyError)
                })
            }
            PulseFixtureMutation::Unsupported => Ok(1),
        }
    }

    fn coalesce(
        &self,
        previous: &PulseFixtureMutation,
        incoming: &PulseFixtureMutation,
        context: HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<PulseFixtureMutation>, Self::Error> {
        match (previous, incoming) {
            (
                PulseFixtureMutation::RenameTrack {
                    track_id, before, ..
                },
                PulseFixtureMutation::RenameTrack {
                    track_id: incoming_track_id,
                    after,
                    ..
                },
            ) if track_id == incoming_track_id => {
                if before == after {
                    Ok(HistoryCoalesce::Remove)
                } else {
                    Ok(HistoryCoalesce::Replace(
                        PulseFixtureMutation::RenameTrack {
                            track_id: *track_id,
                            before: before.clone(),
                            after: after.clone(),
                        },
                    ))
                }
            }
            _ => Ok(match context {
                HistoryCoalesceContext::Adjacent => HistoryCoalesce::KeepSeparate,
                HistoryCoalesceContext::Group { .. } => {
                    let mut mutations = match previous {
                        PulseFixtureMutation::Compound { mutations } => mutations.clone(),
                        previous => vec![previous.clone()],
                    };
                    mutations.push(incoming.clone());
                    HistoryCoalesce::Replace(PulseFixtureMutation::Compound { mutations })
                }
            }),
        }
    }
}

pub(crate) fn rename(track_id: u32, before: &str, after: &str) -> PulseFixtureMutation {
    PulseFixtureMutation::RenameTrack {
        track_id,
        before: before.to_owned(),
        after: after.to_owned(),
    }
}

#[test]
fn pulse_shaped_inverse_coalesce_and_removal_keep_payload_policy_local() {
    let policy = PulseFixturePolicy;
    let inverse = policy.inverse(&rename(7, "Drums", "Kit")).unwrap();
    assert_eq!(inverse, rename(7, "Kit", "Drums"));
    assert_eq!(
        policy.inverse(&PulseFixtureMutation::Unsupported),
        Err(PulseFixturePolicyError)
    );

    let mut history = LinearHistory::new(history_id("history:pulse"), HistoryLimits::default());
    let added = history
        .record_applied(
            record(
                0,
                "entry:0001",
                metadata("Rename track", "track:rename"),
                rename(7, "Drums", "Kit"),
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(
        added.outcome(),
        &HistoryRecordOutcome::Added {
            entry_id: entry_id("entry:0001"),
            sequence: HistoryEntrySequence::FIRST,
        }
    );

    let replaced = history
        .record_applied(
            record(
                1,
                "entry:0002",
                metadata("Rename track to Beats", "track:rename"),
                rename(7, "Kit", "Beats"),
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(
        replaced.outcome(),
        &HistoryRecordOutcome::Replaced {
            entry_id: entry_id("entry:0001"),
            sequence: HistoryEntrySequence::FIRST,
        }
    );
    assert_eq!(history.applied().len(), 1);
    assert_eq!(
        history.current().unwrap().payload(),
        &rename(7, "Drums", "Beats")
    );
    assert_eq!(history.current().unwrap().committed_revision().get(), 2);
    assert_eq!(history.next_sequence().get(), 2);

    let removed = history
        .record_applied(
            record(
                2,
                "entry:0003",
                metadata("Restore track name", "track:rename"),
                rename(7, "Beats", "Drums"),
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(
        removed.outcome(),
        &HistoryRecordOutcome::Removed {
            entry_id: entry_id("entry:0001"),
            sequence: HistoryEntrySequence::FIRST,
        }
    );
    assert!(history.applied().is_empty());
    assert_eq!(history.revision().get(), 3);
}

#[test]
fn pulse_shaped_import_preserves_full_linear_shape_and_divergence_clears_future() {
    let state = LinearHistoryState::new(
        history_id("history:pulse"),
        HistoryRevision::new(5),
        HistoryEntrySequence::new(4).unwrap(),
        vec![entry(
            "entry:0001",
            "Create track",
            "track:create",
            1,
            1,
            PulseFixtureMutation::RestoreTrack {
                track_id: 1,
                snapshot: "Bass".to_owned(),
            },
        )],
        vec![
            entry(
                "entry:0003",
                "Rename track",
                "track:rename",
                3,
                3,
                rename(1, "Bass", "Sub"),
            ),
            entry(
                "entry:0002",
                "Delete track",
                "track:delete",
                2,
                2,
                PulseFixtureMutation::DeleteTrack {
                    track_id: 2,
                    snapshot: "Keys".to_owned(),
                },
            ),
        ],
    );
    let mut history = LinearHistory::from_state(HistoryLimits::default(), state).unwrap();

    assert_eq!(history.applied().len(), 1);
    assert_eq!(history.future().len(), 2);
    assert_eq!(
        history.next_undo().unwrap().metadata().label().as_str(),
        "Create track"
    );
    assert_eq!(
        history.next_redo().unwrap().metadata().label().as_str(),
        "Delete track"
    );

    let result = history
        .record_applied(
            record(
                5,
                "entry:0004",
                metadata("Delete track", "track:delete"),
                PulseFixtureMutation::DeleteTrack {
                    track_id: 9,
                    snapshot: "FX".to_owned(),
                },
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    assert_eq!(
        result.cleared_future(),
        &[entry_id("entry:0002"), entry_id("entry:0003")]
    );
    assert!(history.future().is_empty());
    assert_eq!(history.applied().len(), 2);
    assert_eq!(
        history.current().unwrap().entry_id(),
        &entry_id("entry:0004")
    );

    let structural = history.clone().into_state();
    let restored = LinearHistory::from_state(HistoryLimits::default(), structural.clone()).unwrap();
    assert_eq!(restored.into_state(), structural);
}

#[test]
fn standalone_noop_is_explicit_and_does_not_destroy_future() {
    let state = LinearHistoryState::new(
        history_id("history:pulse"),
        HistoryRevision::new(2),
        HistoryEntrySequence::new(3).unwrap(),
        vec![entry(
            "entry:0001",
            "Rename track",
            "track:rename",
            1,
            1,
            rename(1, "A", "B"),
        )],
        vec![entry(
            "entry:0002",
            "Rename track",
            "track:rename",
            2,
            2,
            rename(2, "C", "D"),
        )],
    );
    let mut history = LinearHistory::from_state(HistoryLimits::default(), state).unwrap();
    let before = history.clone();

    let result = history
        .record_applied(
            record(
                2,
                "entry:noop",
                metadata("No change", "track:rename"),
                rename(1, "B", "B"),
            ),
            &PulseFixturePolicy,
        )
        .unwrap();

    assert_eq!(
        result.outcome(),
        &HistoryRecordOutcome::IgnoredNoOp {
            entry_id: entry_id("entry:noop"),
        }
    );
    assert_eq!(result.previous_revision(), result.committed_revision());
    assert!(result.cleared_future().is_empty());
    assert_eq!(history, before);
}

#[test]
fn donor_default_limit_prunes_oldest_with_explicit_baseline_evidence() {
    assert_eq!(HistoryLimits::default().maximum_entries(), 100);

    let limits = HistoryLimits::new(1, 1_024, 1_024).unwrap();
    let mut history = LinearHistory::new(history_id("history:pulse"), limits);
    history
        .record_applied(
            record(
                0,
                "entry:0001",
                metadata("Delete track", "track:delete"),
                PulseFixtureMutation::DeleteTrack {
                    track_id: 1,
                    snapshot: "Bass".to_owned(),
                },
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    let result = history
        .record_applied(
            record(
                1,
                "entry:0002",
                metadata("Delete another track", "track:delete"),
                PulseFixtureMutation::DeleteTrack {
                    track_id: 2,
                    snapshot: "Keys".to_owned(),
                },
            ),
            &PulseFixturePolicy,
        )
        .unwrap();
    assert_eq!(history.applied().len(), 1);
    assert_eq!(
        history.current().unwrap().entry_id(),
        &entry_id("entry:0002")
    );
    assert_eq!(
        result.pruning().advanced_baseline()[0].entry_id(),
        &entry_id("entry:0001")
    );
    assert_eq!(history.retained_baseline().pruned_entry_count(), 1);
}
