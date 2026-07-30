use crate::CommandDiscoveryRecord;
use serde::Serialize;

/// One deterministic command search result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommandSearchHit {
    /// Discovery record from the sealed registry.
    pub record: CommandDiscoveryRecord,
    /// Canonical rank; lower is better.
    pub score: u32,
}

pub(crate) fn search_records<'registry>(
    records: impl Iterator<Item = &'registry CommandDiscoveryRecord>,
    query: &str,
) -> Vec<CommandSearchHit> {
    let terms: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut hits: Vec<_> = records
        .filter_map(|record| {
            score_record(record, &terms).map(|score| CommandSearchHit {
                record: record.clone(),
                score,
            })
        })
        .collect();
    hits.sort_by(|left, right| {
        left.score
            .cmp(&right.score)
            .then_with(|| {
                left.record
                    .label
                    .to_lowercase()
                    .cmp(&right.record.label.to_lowercase())
            })
            .then_with(|| left.record.id.cmp(&right.record.id))
    });
    hits
}

fn score_record(record: &CommandDiscoveryRecord, terms: &[String]) -> Option<u32> {
    if terms.is_empty() {
        return Some(0);
    }
    let id = record.id.as_str();
    let label = record.label.to_lowercase();
    let description = record
        .description
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    let categories: Vec<_> = record
        .category_path
        .iter()
        .map(|category| category.as_str())
        .collect();
    let keywords: Vec<_> = record
        .keywords
        .iter()
        .map(|keyword| keyword.as_str().to_lowercase())
        .collect();

    terms
        .iter()
        .map(|term| {
            if label == *term {
                Some(0)
            } else if label.starts_with(term) {
                Some(10)
            } else if label.contains(term) {
                Some(20)
            } else if keywords.iter().any(|keyword| keyword == term) {
                Some(30)
            } else if keywords.iter().any(|keyword| keyword.starts_with(term)) {
                Some(40)
            } else if keywords.iter().any(|keyword| keyword.contains(term)) {
                Some(50)
            } else if categories.iter().any(|category| category.contains(term)) {
                Some(60)
            } else if id.contains(term) {
                Some(70)
            } else if description.contains(term) {
                Some(80)
            } else {
                None
            }
        })
        .try_fold(0_u32, |score, term_score| {
            term_score.map(|term_score| score.saturating_add(term_score))
        })
}
