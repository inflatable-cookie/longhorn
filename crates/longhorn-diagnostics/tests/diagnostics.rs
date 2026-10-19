//! Contract 026 through the public API only.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use longhorn_diagnostics::{Fact, Facts, Section, render};
use longhorn_redact::{REDACTED, RedactionRules, Redactor};
use longhorn_support_log::{Level, RingLog};

const GOLDEN: &str = include_str!("fixtures/golden-bundle.txt");
const TOKEN: &str = "ghp_A1b2C3d4E5f6G7h8J9k0LmNoPqRsTuVwXyZ1";
const REFRESH: &str = "refresh-token-9f8e7d6c5b4a";
const KEYCHAIN: &str = "keychain-secret-value-42";
const INSTANT: u64 = 1_775_000_000;

fn redactor() -> Redactor {
    Redactor::new(Vec::new(), RedactionRules::common())
}

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn statement() -> String {
    "Redaction: applied before this text was created.".to_owned()
}

/// Facts shaped like the donor's fixture, with neutral text.
fn facts() -> Facts {
    Facts {
        generated_at: at(INSTANT),
        title: "Support diagnostics".to_owned(),
        redaction_statement: statement(),
        sections: vec![
            Section::Facts {
                heading: "Application".to_owned(),
                facts: vec![
                    Fact::new("Version", "0.1.13"),
                    Fact::new("Channel", "production"),
                    Fact::new(
                        "Updates",
                        "installed 0.1.13 on production; availability up to date; staged none",
                    ),
                ],
            },
            Section::Facts {
                heading: "System".to_owned(),
                facts: vec![
                    Fact::new("OS", "TestOS 15.1"),
                    Fact::new("Architecture", "arm64"),
                ],
            },
            Section::Facts {
                heading: "Setup".to_owned(),
                facts: vec![
                    Fact::new("You", "complete"),
                    Fact::new("Sign-in", "complete"),
                    Fact::new("Workspace", "current"),
                    Fact::new("Ready", "pending"),
                ],
            },
            Section::Items {
                heading: "Tools".to_owned(),
                items: vec![
                    "Teapot: 0.9.1 at /usr/local/bin/teapot; signed in; catalogue loaded; ready"
                        .to_owned(),
                ],
                empty: "No harness route is registered in this build.".to_owned(),
            },
            Section::Facts {
                heading: "Connection".to_owned(),
                facts: vec![Fact::new("State", "connected: the service is connected.")],
            },
            Section::Facts {
                heading: "Workspace".to_owned(),
                facts: vec![
                    Fact::new("Path", "/workspace/teapot"),
                    Fact::new("Branch", "client/teapot"),
                    Fact::new("Remote", "example.com/acme/teapot"),
                    Fact::new(
                        "Synchronization",
                        "in sync with the remote and no local changes",
                    ),
                ],
            },
            Section::Items {
                heading: "Attention".to_owned(),
                items: vec!["Publish — delivery is unknown".to_owned()],
                empty: "Nothing needs attention.".to_owned(),
            },
            Section::Records {
                heading: "Recent log (up to the last 100 entries)".to_owned(),
                records: Vec::new(),
                empty: "No entries recorded this launch.".to_owned(),
            },
        ],
    }
}

fn fact_value<'a>(input: &'a mut Facts, heading: &str, index: usize) -> &'a mut String {
    let Some(Section::Facts { facts, .. }) = input.sections.iter_mut().find(
        |section| matches!(section, Section::Facts { heading: found, .. } if found == heading),
    ) else {
        panic!("no {heading} section");
    };
    &mut facts[index].value
}

// --- the three donor tests, ported ----------------------------------------

#[test]
fn bundle_carries_every_support_section() {
    let mut input = facts();
    let log = RingLog::new(8, 4096, redactor());
    log.record(Level::Error, "source", "connection refused");
    if let Some(Section::Records { records, .. }) = input.sections.last_mut() {
        *records = log.tail(100);
    }
    let bundle = render(&redactor(), &input);
    // The fact rows are column-aligned, so compare whitespace-normalized text.
    let normalized = bundle.text.split_whitespace().collect::<Vec<_>>().join(" ");

    for expected in [
        "Support diagnostics",
        "Generated: 2026-03-31T23:33:20.000Z",
        "Redaction: applied before this text was created.",
        "Application",
        "Version: 0.1.13",
        "Channel: production",
        "System",
        "TestOS 15.1",
        "Architecture: arm64",
        "Setup",
        "You: complete",
        "Sign-in: complete",
        "Workspace: current",
        "Ready: pending",
        "Tools",
        "Teapot: 0.9.1",
        "Connection",
        "connected: the service is connected.",
        "Workspace",
        "/workspace/teapot",
        "client/teapot",
        "example.com/acme/teapot",
        "Attention",
        "Publish — delivery is unknown",
        "Recent log (up to the last 100 entries)",
        "[error] source: connection refused",
    ] {
        assert!(normalized.contains(expected), "missing {expected:?}");
    }
    assert_eq!(bundle.redactions, 0);
}

#[test]
fn bundle_says_plainly_when_a_section_has_nothing() {
    let mut input = facts();
    for section in &mut input.sections {
        match section {
            Section::Items { items, .. } => items.clear(),
            Section::Records { records, .. } => records.clear(),
            Section::Facts { .. } => {}
        }
    }
    let bundle = render(&redactor(), &input);
    assert!(bundle.text.contains("Nothing needs attention."));
    assert!(
        bundle
            .text
            .contains("No harness route is registered in this build.")
    );
    assert!(bundle.text.contains("No entries recorded this launch."));
}

#[test]
fn seeded_secrets_never_reach_the_bundle_and_are_counted() {
    let mut input = facts();
    *fact_value(&mut input, "Application", 0) = format!("0.1.13 ({TOKEN})");
    *fact_value(&mut input, "Connection", 0) = format!("connected with {REFRESH}");
    *fact_value(&mut input, "Workspace", 2) =
        format!("https://x-access-token:{KEYCHAIN}@teapot.example/acme/teapot");
    let seeds = Redactor::new(
        vec![TOKEN.to_owned(), REFRESH.to_owned()],
        RedactionRules::common(),
    );
    let bundle = render(&seeds, &input);

    for secret in [TOKEN, REFRESH, KEYCHAIN] {
        assert!(!bundle.text.contains(secret), "leaked {secret}");
    }
    assert!(bundle.text.contains(REDACTED));
    // Exactly the seeded token, the seeded refresh token, and the unseeded
    // URL userinfo are replaced; the keychain value is caught by shape alone.
    assert_eq!(
        bundle.redactions, 3,
        "expected three redaction markers, got {}",
        bundle.redactions
    );
}

// --- golden ----------------------------------------------------------------

#[test]
fn golden_facts_render_the_committed_text_byte_for_byte() {
    let bundle = render(&redactor(), &facts());
    assert_eq!(bundle.generated_at, "2026-03-31T23:33:20.000Z");
    assert_eq!(bundle.redactions, 0);
    assert_eq!(bundle.text, GOLDEN);
}

// --- redact-last -----------------------------------------------------------

#[test]
fn secret_split_across_label_and_value_is_caught() {
    let label = "LongLabelOverColumn";
    let value = "value-with-secret-123";
    let mut input = facts();
    input.sections = vec![Section::Facts {
        heading: "Application".to_owned(),
        facts: vec![Fact::new(label, value)],
    }];
    let seeds = Redactor::new(
        vec![format!("{label}:{value}")],
        RedactionRules::exact_only(),
    );
    let bundle = render(&seeds, &input);

    // The seed only exists across the colon: neither half alone matches.
    assert!(bundle.text.contains("  [redacted]\n"));
    assert!(!bundle.text.contains(label));
    assert!(!bundle.text.contains(value));
    assert_eq!(bundle.redactions, 1);
}

#[test]
fn seed_spanning_a_raw_line_break_is_not_caught_and_is_pinned() {
    let mut input = facts();
    input.sections = vec![Section::Facts {
        heading: "Application".to_owned(),
        facts: vec![Fact::new("Version", "line-one-secret\nline-two-secret")],
    }];
    let seeds = Redactor::new(
        vec!["line-one-secret\nline-two-secret".to_owned()],
        RedactionRules::exact_only(),
    );
    let bundle = render(&seeds, &input);

    // Contract 026's listed delta: the text is escaped before the single
    // redact pass, so this seed — whose match depends on a raw line break —
    // is not caught. The halves stay, escaped onto one line. Shape rules
    // need no raw break and still catch their spans (see the PEM test).
    assert_eq!(bundle.redactions, 0);
    assert!(
        bundle
            .text
            .contains("  Version:           line-one-secret\\nline-two-secret\n")
    );
    assert_eq!(bundle.text.lines().count(), 6);
}

#[test]
fn pem_block_in_a_value_is_redacted_as_one_marker() {
    let mut input = facts();
    input.sections = vec![Section::Facts {
        heading: "Application".to_owned(),
        facts: vec![Fact::new(
            "Version",
            "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n-----END PRIVATE KEY-----",
        )],
    }];
    let bundle = render(&redactor(), &input);
    assert!(!bundle.text.contains("MIIEvQ"));
    assert!(!bundle.text.contains("BEGIN"));
    assert_eq!(bundle.redactions, 1);
    // The escaped block sits on the row's one line, so the layout holds.
    assert_eq!(bundle.text.lines().count(), 6);
}

#[test]
fn seeded_secrets_in_every_section_kind_are_counted() {
    let mut input = facts();
    input.title = "Support diagnostics seed-title-x1".to_owned();
    input.redaction_statement = "Redaction: seed-statement-x2".to_owned();
    *fact_value(&mut input, "Application", 0) = "0.1.13 seed-value-x3".to_owned();
    for section in &mut input.sections {
        if let Section::Items { heading, items, .. } = section
            && heading == "Tools"
        {
            items.push("seed-item-x4".to_owned());
        }
    }
    // The log's own redactor is exact-only, so the token is stored raw and
    // only the render pass over the assembled text catches it.
    let log = RingLog::new(
        8,
        4096,
        Redactor::new(Vec::new(), RedactionRules::exact_only()),
    );
    log.record(
        Level::Error,
        "source",
        "lost ghp_Qq1wW2eE3rR4tT5yY6uU7iI8oO9pP0aA",
    );
    if let Some(Section::Records { records, .. }) = input.sections.last_mut() {
        *records = log.tail(10);
    }
    let seeds = Redactor::new(
        vec![
            "seed-title-x1".to_owned(),
            "seed-statement-x2".to_owned(),
            "seed-value-x3".to_owned(),
            "seed-item-x4".to_owned(),
        ],
        RedactionRules::common(),
    );
    let bundle = render(&seeds, &input);

    for secret in [
        "seed-title-x1",
        "seed-statement-x2",
        "seed-value-x3",
        "seed-item-x4",
        "ghp_Qq1wW2eE3rR4tT5yY6uU7iI8oO9pP0aA",
    ] {
        assert!(!bundle.text.contains(secret), "leaked {secret}");
    }
    // Four seeded secrets plus the token the shape rule catches in the
    // record row.
    assert_eq!(bundle.redactions, 5);
}

// --- records ---------------------------------------------------------------

#[test]
fn record_tail_renders_one_row_per_record_oldest_first() {
    let log = RingLog::new(8, 4096, redactor());
    log.record_at(at(INSTANT), Level::Error, "source", "connection refused");
    log.record_at(at(INSTANT + 60), Level::Warning, "source", "unsynced");
    log.record_at(at(INSTANT + 120), Level::Info, "source", "generated");
    let input = Facts {
        generated_at: at(INSTANT),
        title: "Support diagnostics".to_owned(),
        redaction_statement: statement(),
        sections: vec![Section::Records {
            heading: "Recent log".to_owned(),
            records: log.tail(10),
            empty: "No entries recorded this launch.".to_owned(),
        }],
    };
    let bundle = render(&redactor(), &input);
    assert!(
        bundle
            .text
            .contains("  2026-03-31T23:33:20.000Z [error] source: connection refused\n")
    );
    assert!(
        bundle
            .text
            .contains("  2026-03-31T23:34:20.000Z [warning] source: unsynced\n")
    );
    assert!(
        bundle
            .text
            .contains("  2026-03-31T23:35:20.000Z [info] source: generated\n")
    );
    assert!(bundle.text.ends_with("[info] source: generated\n"));
}

// --- the layout rules --------------------------------------------------------

#[test]
fn label_at_or_over_the_column_gets_no_padding() {
    let input = Facts {
        generated_at: at(INSTANT),
        title: "Support diagnostics".to_owned(),
        redaction_statement: statement(),
        sections: vec![Section::Facts {
            heading: "Application".to_owned(),
            facts: vec![
                Fact::new("BoundaryLabel12345", "exactly eighteen bytes"),
                Fact::new("LongerLabelOverColumn", "over the column"),
            ],
        }],
    };
    let bundle = render(&redactor(), &input);
    assert!(
        bundle
            .text
            .contains("  BoundaryLabel12345:exactly eighteen bytes\n")
    );
    assert!(
        bundle
            .text
            .contains("  LongerLabelOverColumn:over the column\n")
    );
}

#[test]
fn multi_byte_labels_pad_by_bytes() {
    let input = Facts {
        generated_at: at(INSTANT),
        title: "Support diagnostics".to_owned(),
        redaction_statement: statement(),
        sections: vec![Section::Facts {
            heading: "System".to_owned(),
            facts: vec![
                // 8 bytes in 6 characters: 10 spaces of padding.
                Fact::new("Résumé", "done"),
                // 6 bytes in 2 characters: 12 spaces of padding.
                Fact::new("設定", "kept"),
            ],
        }],
    };
    let bundle = render(&redactor(), &input);
    let résumé = format!("  Résumé:{}done\n", " ".repeat(18 - "Résumé".len()));
    let flat = format!("  設定:{}kept\n", " ".repeat(18 - "設定".len()));
    assert!(bundle.text.contains(&résumé), "row missing: {résumé:?}");
    assert!(bundle.text.contains(&flat), "row missing: {flat:?}");
}

#[test]
fn interior_line_breaks_stay_on_one_line() {
    let input = Facts {
        generated_at: at(INSTANT),
        title: "Titled\nline".to_owned(),
        redaction_statement: "Statement\rsplit".to_owned(),
        sections: vec![
            Section::Facts {
                heading: "Application".to_owned(),
                facts: vec![Fact::new("Note", "one\r\ntwo")],
            },
            Section::Items {
                heading: "Attention".to_owned(),
                items: vec!["item\rbreak".to_owned()],
                empty: "Nothing needs attention.".to_owned(),
            },
        ],
    };
    let bundle = render(&redactor(), &input);
    assert!(bundle.text.contains("Titled\\nline\n"));
    assert!(bundle.text.contains("Statement\\rsplit\n"));
    assert!(bundle.text.contains("  Note:              one\\r\\ntwo\n"));
    assert!(bundle.text.contains("  - item\\rbreak\n"));
    // The escapes are the two characters `\r` and `\n`; no raw break remains.
    assert!(!bundle.text.contains('\r'));
    assert_eq!(bundle.text.lines().count(), 9);
}

#[test]
fn empty_sections_render_their_heading_and_their_empty_sentence() {
    let input = Facts {
        generated_at: at(INSTANT),
        title: "Support diagnostics".to_owned(),
        redaction_statement: statement(),
        sections: vec![
            Section::Facts {
                heading: "Connection".to_owned(),
                facts: Vec::new(),
            },
            Section::Items {
                heading: "Tools".to_owned(),
                items: Vec::new(),
                empty: "No harness route is registered in this build.".to_owned(),
            },
            Section::Records {
                heading: "Recent log".to_owned(),
                records: Vec::new(),
                empty: "No entries recorded this launch.".to_owned(),
            },
        ],
    };
    let bundle = render(&redactor(), &input);
    assert!(bundle.text.contains("Connection\n\n"));
    assert!(
        bundle
            .text
            .contains("Tools\n  - No harness route is registered in this build.\n")
    );
    assert!(
        bundle
            .text
            .contains("Recent log\n  - No entries recorded this launch.\n")
    );
}

#[test]
fn rendering_twice_gives_the_same_text() {
    let redactor = redactor();
    let first = render(&redactor, &facts());
    let second = render(&redactor, &facts());
    assert_eq!(first, second);
}
