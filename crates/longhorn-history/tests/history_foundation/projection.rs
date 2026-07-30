use longhorn_core::HistoryRevision;
use longhorn_history::{
    HistoryEntryPosition, HistoryEntrySequence, HistoryLimits, HistoryNavigationLimits,
    HistoryPageRequest, HistoryPageRequestError, HistoryProjectionError, HistoryProjectionLimits,
    HistoryRetainedBaseline, LinearHistory, LinearHistoryState,
    MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
};

use crate::support::*;

#[test]
fn summary_and_pages_expose_authoritative_past_current_and_future() {
    let state = LinearHistoryState::new(
        history_id("history:projection"),
        HistoryRevision::new(5),
        HistoryEntrySequence::new(6).unwrap(),
        vec![
            entry("entry:1", "One", "document:step", 1, 1, 1_u8),
            entry("entry:2", "Two", "document:step", 2, 2, 2_u8),
            entry("entry:3", "Three", "document:step", 3, 3, 3_u8),
        ],
        vec![
            entry("entry:5", "Five", "document:step", 5, 5, 5_u8),
            entry("entry:4", "Four", "document:step", 4, 4, 4_u8),
        ],
    );
    let history = LinearHistory::from_state_with_runtime_limits(
        HistoryLimits::default(),
        HistoryNavigationLimits::default(),
        HistoryProjectionLimits::new(2).unwrap(),
        state,
    )
    .unwrap();

    let summary = history.project_summary().unwrap();
    assert_eq!(summary.undo_depth(), 3);
    assert_eq!(summary.redo_depth(), 2);
    assert_eq!(summary.current_entry_id(), Some(&entry_id("entry:3")));
    assert_eq!(summary.next_undo_label().unwrap().as_str(), "Three");
    assert_eq!(summary.next_redo_label().unwrap().as_str(), "Four");
    assert_eq!(summary.retained_entry_count(), 5);
    assert_eq!(summary.retained_encoded_weight(), 5);

    let first = history
        .project_page(HistoryPageRequest::new(0, 2).unwrap())
        .unwrap();
    assert_eq!(
        first
            .entries()
            .iter()
            .map(|entry| (entry.entry_id().as_str(), entry.position()))
            .collect::<Vec<_>>(),
        vec![
            ("entry:5", HistoryEntryPosition::Future),
            ("entry:4", HistoryEntryPosition::Future),
        ]
    );
    assert!(!first.truncated_before());
    assert!(first.truncated_after());

    let middle = history
        .project_page(HistoryPageRequest::new(2, 2).unwrap())
        .unwrap();
    assert_eq!(
        middle
            .entries()
            .iter()
            .map(|entry| (entry.entry_id().as_str(), entry.position()))
            .collect::<Vec<_>>(),
        vec![
            ("entry:3", HistoryEntryPosition::Current),
            ("entry:2", HistoryEntryPosition::Past),
        ]
    );
    assert!(middle.truncated_before());
    assert!(middle.truncated_after());

    let last = history
        .project_page(HistoryPageRequest::new(4, 1).unwrap())
        .unwrap();
    assert_eq!(last.entries()[0].entry_id(), &entry_id("entry:1"));
    assert!(last.truncated_before());
    assert!(!last.truncated_after());
    assert_eq!(
        history.project_page(HistoryPageRequest::new(0, 3).unwrap()),
        Err(HistoryProjectionError::PageTooLarge {
            maximum: 2,
            actual: 3,
        })
    );
}

#[test]
fn projection_bounds_offsets_and_carries_retained_baseline() {
    assert_eq!(
        HistoryPageRequest::new(0, 0),
        Err(HistoryPageRequestError::Zero)
    );
    assert_eq!(
        HistoryPageRequest::new(0, MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE + 1),
        Err(HistoryPageRequestError::TooLarge {
            maximum: MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE,
            actual: MAXIMUM_HISTORY_PROJECTION_PAGE_SIZE + 1,
        })
    );

    let baseline = HistoryRetainedBaseline::new(
        2,
        9,
        Some(entry_id("entry:2")),
        Some(HistoryEntrySequence::new(2).unwrap()),
    );
    let state = LinearHistoryState::with_retained_baseline(
        history_id("history:baseline"),
        HistoryRevision::new(4),
        HistoryEntrySequence::new(5).unwrap(),
        baseline.clone(),
        vec![
            entry("entry:3", "Three", "document:step", 3, 3, 3_u8),
            entry("entry:4", "Four", "document:step", 4, 4, 4_u8),
        ],
        Vec::new(),
    );
    let history = LinearHistory::from_state(HistoryLimits::default(), state).unwrap();
    assert_eq!(
        history.project_summary().unwrap().retained_baseline(),
        &baseline
    );
    let page = history
        .project_page(HistoryPageRequest::new(2, 1).unwrap())
        .unwrap();
    assert!(page.entries().is_empty());
    assert!(page.truncated_before());
    assert!(!page.truncated_after());
    assert_eq!(page.retained_baseline(), &baseline);
    assert_eq!(
        history.project_page(HistoryPageRequest::new(3, 1).unwrap()),
        Err(HistoryProjectionError::OffsetOutOfRange {
            maximum: 2,
            actual: 3,
        })
    );
}
