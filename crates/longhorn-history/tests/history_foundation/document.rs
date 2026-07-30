use std::{convert::Infallible, error::Error, fmt};

use longhorn_history::{
    HistoryCoalesce, HistoryLimits, HistoryPolicy, HistoryRecordOutcome, LinearHistory,
};

use crate::support::*;

#[derive(Clone, Debug, Eq, PartialEq)]
enum DocumentMutation {
    SetTitle { before: String, after: String },
    Insert { index: usize, value: String },
    Remove { index: usize, value: String },
}

struct DocumentPolicy;

impl HistoryPolicy<DocumentMutation> for DocumentPolicy {
    type Error = Infallible;

    fn inverse(&self, payload: &DocumentMutation) -> Result<DocumentMutation, Self::Error> {
        Ok(match payload {
            DocumentMutation::SetTitle { before, after } => DocumentMutation::SetTitle {
                before: after.clone(),
                after: before.clone(),
            },
            DocumentMutation::Insert { index, value } => DocumentMutation::Remove {
                index: *index,
                value: value.clone(),
            },
            DocumentMutation::Remove { index, value } => DocumentMutation::Insert {
                index: *index,
                value: value.clone(),
            },
        })
    }

    fn is_noop(&self, payload: &DocumentMutation) -> bool {
        matches!(payload, DocumentMutation::SetTitle { before, after } if before == after)
    }

    fn encoded_weight(&self, _: &DocumentMutation) -> Result<u64, Self::Error> {
        Ok(1)
    }

    fn coalesce(
        &self,
        previous: &DocumentMutation,
        incoming: &DocumentMutation,
        _: longhorn_history::HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<DocumentMutation>, Self::Error> {
        Ok(match (previous, incoming) {
            (
                DocumentMutation::SetTitle { before, .. },
                DocumentMutation::SetTitle { after, .. },
            ) if before == after => HistoryCoalesce::Remove,
            (
                DocumentMutation::SetTitle { before, .. },
                DocumentMutation::SetTitle { after, .. },
            ) => HistoryCoalesce::Replace(DocumentMutation::SetTitle {
                before: before.clone(),
                after: after.clone(),
            }),
            _ => HistoryCoalesce::KeepSeparate,
        })
    }
}

#[test]
fn non_editor_document_uses_the_same_typed_record_api() {
    let mut history = LinearHistory::new(history_id("history:document"), HistoryLimits::default());
    let policy = DocumentPolicy;

    history
        .record_applied(
            record(
                0,
                "entry:title",
                metadata("Rename document", "document:title"),
                DocumentMutation::SetTitle {
                    before: "Draft".to_owned(),
                    after: "Plan".to_owned(),
                },
            ),
            &policy,
        )
        .unwrap();
    let coalesced = history
        .record_applied(
            record(
                1,
                "entry:title-2",
                metadata("Name document", "document:title"),
                DocumentMutation::SetTitle {
                    before: "Plan".to_owned(),
                    after: "Roadmap".to_owned(),
                },
            ),
            &policy,
        )
        .unwrap();
    assert!(matches!(
        coalesced.outcome(),
        HistoryRecordOutcome::Replaced { .. }
    ));
    assert_eq!(
        history.current().unwrap().payload(),
        &DocumentMutation::SetTitle {
            before: "Draft".to_owned(),
            after: "Roadmap".to_owned(),
        }
    );

    history
        .record_applied(
            record(
                2,
                "entry:insert",
                metadata("Insert item", "document:insert"),
                DocumentMutation::Insert {
                    index: 0,
                    value: "Scope".to_owned(),
                },
            ),
            &policy,
        )
        .unwrap();
    assert_eq!(history.applied().len(), 2);
    assert_eq!(
        policy
            .inverse(history.current().unwrap().payload())
            .unwrap(),
        DocumentMutation::Remove {
            index: 0,
            value: "Scope".to_owned(),
        }
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RejectCoalesce;

impl fmt::Display for RejectCoalesce {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("coalescing rejected")
    }
}

impl Error for RejectCoalesce {}

struct RejectingPolicy;

impl HistoryPolicy<DocumentMutation> for RejectingPolicy {
    type Error = RejectCoalesce;

    fn inverse(&self, _: &DocumentMutation) -> Result<DocumentMutation, Self::Error> {
        Err(RejectCoalesce)
    }

    fn is_noop(&self, _: &DocumentMutation) -> bool {
        false
    }

    fn encoded_weight(&self, _: &DocumentMutation) -> Result<u64, Self::Error> {
        Err(RejectCoalesce)
    }

    fn coalesce(
        &self,
        _: &DocumentMutation,
        _: &DocumentMutation,
        _: longhorn_history::HistoryCoalesceContext<'_>,
    ) -> Result<HistoryCoalesce<DocumentMutation>, Self::Error> {
        Err(RejectCoalesce)
    }
}

#[test]
fn policy_rejection_preserves_exact_structural_state() {
    let mut history = LinearHistory::new(history_id("history:document"), HistoryLimits::default());
    history
        .record_applied(
            record(
                0,
                "entry:title",
                metadata("Rename document", "document:title"),
                DocumentMutation::SetTitle {
                    before: "A".to_owned(),
                    after: "B".to_owned(),
                },
            ),
            &DocumentPolicy,
        )
        .unwrap();
    let before = history.clone();

    let result = history.record_applied(
        record(
            1,
            "entry:title-2",
            metadata("Rename document", "document:title"),
            DocumentMutation::SetTitle {
                before: "B".to_owned(),
                after: "C".to_owned(),
            },
        ),
        &RejectingPolicy,
    );
    assert!(matches!(
        result,
        Err(longhorn_history::HistoryRecordError::Policy(RejectCoalesce))
    ));
    assert_eq!(history, before);
}
