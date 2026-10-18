//! Std-only secret redaction for desktop support text.
//!
//! This crate implements Longhorn contract 024. A [`Redactor`] replaces every
//! occurrence of the exact secrets a product seeds, and every span its
//! [`RedactionRules`] recognise, with the one marker [`REDACTED`].
//!
//! Redaction runs in passes. Each pass treats the markers already in its
//! input as protected, collects candidate spans from every enabled rule,
//! clips them around the protected markers, merges overlapping or touching
//! spans, and replaces each merged span with one marker. Passes repeat until
//! one finds nothing, so a second call never changes the text.
//!
//! The crate owns the engine and the [`RedactionRules::common`] vocabulary of
//! published vendor formats. A product decides which secrets to seed, which
//! rules to enable, and where output is redacted.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod rules;

use std::collections::BTreeSet;
use std::ops::Range;

/// The one replacement marker.
pub const REDACTED: &str = "[redacted]";

/// Shortest secret, in characters, that [`Redactor::new`] accepts. Shorter
/// values would mangle ordinary prose without protecting anything.
const MIN_SECRET_CHARS: usize = 4;

/// Which shape rules run, and the vocabulary they use.
#[derive(Clone, Debug)]
pub struct RedactionRules {
    /// Token prefixes, matched case-sensitively at a token boundary.
    pub token_prefixes: Vec<String>,
    /// Assignment keys (`key=value`, `key: value`), matched
    /// case-insensitively at a word boundary.
    pub assignment_keys: Vec<String>,
    /// Redacts the credential after `Bearer `.
    pub bearer: bool,
    /// Redacts password-form userinfo in `scheme://user:password@host`.
    pub url_userinfo: bool,
    /// Redacts PEM private-key blocks from the BEGIN line through the END line.
    pub pem_private_keys: bool,
    /// Redacts JSON Web Tokens.
    pub jwt: bool,
}

impl RedactionRules {
    /// Every rule on, with widely published credential formats: the GitHub,
    /// npm, Anthropic, OpenAI, Slack, Google and AWS token prefixes, and
    /// common credential assignment keys.
    #[must_use]
    pub fn common() -> Self {
        Self {
            token_prefixes: owned(COMMON_TOKEN_PREFIXES),
            assignment_keys: owned(COMMON_ASSIGNMENT_KEYS),
            bearer: true,
            url_userinfo: true,
            pem_private_keys: true,
            jwt: true,
        }
    }

    /// No shape rules; exact secrets only.
    #[must_use]
    pub fn exact_only() -> Self {
        Self {
            token_prefixes: Vec::new(),
            assignment_keys: Vec::new(),
            bearer: false,
            url_userinfo: false,
            pem_private_keys: false,
            jwt: false,
        }
    }
}

const COMMON_TOKEN_PREFIXES: &[&str] = &[
    // GitHub
    "github_pat_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "ghd_",
    "gha_",
    // npm
    "npm_",
    // Anthropic and OpenAI
    "sk-ant-",
    "sk-proj-",
    // Slack
    "xoxb-",
    "xoxp-",
    "xoxa-",
    "xapp-",
    // Google
    "ya29.",
    "AIza",
    // AWS access key IDs, in their issued upper case
    "AKIA",
    "ASIA",
];

const COMMON_ASSIGNMENT_KEYS: &[&str] = &[
    "authorization",
    "password",
    "passwd",
    "secret",
    "secret_access_key",
    "aws_secret_access_key",
    "aws_access_key_id",
    "token",
    "auth_token",
    "api_key",
    "apikey",
    "credential",
    "client_secret",
    "cookie",
    "session",
    "_auth",
    "gh_token",
    "npm_token",
    "access_token",
    "refresh_token",
    "private_key",
    "authtoken",
    "_authtoken",
];

fn owned(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// Redacts seeded secrets and rule-matched spans from text.
#[derive(Clone, Debug)]
pub struct Redactor {
    secrets: Vec<String>,
    dropped_secrets: usize,
    rules: RedactionRules,
}

impl Redactor {
    /// Secrets are trimmed and deduplicated. A trimmed value under four
    /// characters is not accepted and is reported in `dropped_secrets`.
    #[must_use]
    pub fn new(secrets: impl IntoIterator<Item = String>, rules: RedactionRules) -> Self {
        let mut accepted = BTreeSet::new();
        let mut dropped_secrets = 0;
        for secret in secrets {
            let secret = secret.trim();
            if secret.chars().count() < MIN_SECRET_CHARS {
                dropped_secrets += 1;
            } else {
                accepted.insert(secret.to_owned());
            }
        }
        Self {
            secrets: accepted.into_iter().collect(),
            dropped_secrets,
            rules,
        }
    }

    /// How many supplied secrets were not accepted. Never the values.
    #[must_use]
    pub fn dropped_secrets(&self) -> usize {
        self.dropped_secrets
    }

    /// Redacts `text`, repeating passes until one finds nothing to replace.
    #[must_use]
    pub fn redact(&self, text: &str) -> Redaction {
        let mut current = text.to_owned();
        let mut replaced = 0;
        loop {
            let spans = self.pass_spans(&current);
            if spans.is_empty() {
                break;
            }
            replaced += spans.len();
            current = replace_spans(&current, &spans);
        }
        let count = current.matches(REDACTED).count();
        Redaction {
            text: current,
            count,
            replaced,
        }
    }

    /// The merged spans one pass replaces: every rule's candidates, clipped
    /// around the protected markers, then unioned.
    fn pass_spans(&self, text: &str) -> Vec<Range<usize>> {
        let protected = text
            .match_indices(REDACTED)
            .map(|(start, marker)| start..start + marker.len())
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        for secret in &self.secrets {
            rules::exact(text, secret, &mut candidates);
        }
        rules::shapes(text, &self.rules, &mut candidates);
        let mut clipped = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            clip(candidate, &protected, &mut clipped);
        }
        merge(clipped)
    }
}

/// The result of one [`Redactor::redact`] call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redaction {
    /// The redacted text.
    pub text: String,
    /// `[redacted]` occurrences in `text`, including any already in the
    /// input.
    pub count: usize,
    /// Markers this call inserted: one per merged span, summed over passes.
    pub replaced: usize,
}

/// Pushes each non-empty run of `candidate` that lies outside the protected
/// markers. `protected` is sorted and non-overlapping.
fn clip(candidate: Range<usize>, protected: &[Range<usize>], out: &mut Vec<Range<usize>>) {
    let mut start = candidate.start;
    for marker in protected {
        if marker.end <= start {
            continue;
        }
        if marker.start >= candidate.end {
            break;
        }
        if marker.start > start {
            out.push(start..marker.start);
        }
        start = marker.end;
    }
    if start < candidate.end {
        out.push(start..candidate.end);
    }
}

/// Unions overlapping or touching spans. Clipped spans never contain marker
/// bytes, so two spans separated by a marker never touch.
fn merge(mut spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    spans.sort_by_key(|span| span.start);
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(spans.len());
    for span in spans {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

/// Replaces each sorted, disjoint span with one marker.
fn replace_spans(text: &str, spans: &[Range<usize>]) -> String {
    let mut out = String::with_capacity(text.len() + spans.len() * REDACTED.len());
    let mut cursor = 0;
    for span in spans {
        out.push_str(&text[cursor..span.start]);
        out.push_str(REDACTED);
        cursor = span.end;
    }
    out.push_str(&text[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::{clip, merge};

    #[test]
    fn clip_splits_around_markers_and_drops_wholly_protected_candidates() {
        let mut out = Vec::new();
        clip(0..20, &[4..8, 10..12], &mut out);
        assert_eq!(out, vec![0..4, 8..10, 12..20]);

        out.clear();
        clip(5..7, &[4..8], &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn merge_joins_overlapping_and_touching_spans_only() {
        assert_eq!(merge(vec![6..9, 0..3, 3..4, 2..3]), vec![0..4, 6..9]);
    }
}
