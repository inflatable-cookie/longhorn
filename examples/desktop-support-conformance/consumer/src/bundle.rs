//! Contract 026 (`longhorn-diagnostics`): `render` over gathered facts,
//! including a ring-log tail, gives the contract's layout, redacts last and
//! counts the markers it produced.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use longhorn_diagnostics::{Fact, Facts, Section, render};
use longhorn_redact::{REDACTED, RedactionRules, Redactor};
use longhorn_support_log::{Level, RingLog};

/// A seeded secret, and the same value the log records carry.
const SEED: &str = "seeded-product-credential";

/// The fact row whose label, colon, padding and value one seeded secret
/// covers. A renderer that redacted each field in isolation would miss it,
/// which is the layout's redact-last claim.
const CROSSING_LABEL: &str = "Path";
const CROSSING_VALUE: &str = "/opt/example";

/// The byte column a fact value starts at, counted after the two-space indent
/// (contract 026's layout rule).
const LABEL_COLUMN: usize = 18;

/// The complete text the contract's layout promises for the facts below.
const EXPECTED: &str = "\
Example Support Bundle
Generated: 2023-11-14T22:13:20.123Z
Values below are redacted by the reporter.

Build
  Version:           1.4.0
  Channel:           stable
  Note:              line one\\nline two

Workspace
  [redacted]

Checks
  - network: ok
  - token=[redacted]

Pending
  - nothing pending

Recent log
  2023-11-14T22:13:18.123Z [warning] install: token=[redacted]
  2023-11-14T22:13:19.123Z [error] clone: remote refused

Earlier log
  - no earlier records
";

fn at(milliseconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(milliseconds)
}

pub(crate) fn run() -> String {
    let redactor = Redactor::new([SEED.to_owned()], RedactionRules::common());

    // The record tail comes from a ring log the host owns, as a real consumer
    // would pass it in.
    let ring = RingLog::new(4, 200, redactor.clone());
    ring.record_at(
        at(1_699_999_998_123),
        Level::Warning,
        "install",
        &format!("token={SEED}"),
    );
    ring.record_at(
        at(1_699_999_999_123),
        Level::Error,
        "clone",
        "remote refused",
    );

    let generated_at = at(1_700_000_000_123);
    let crossing_seed = format!(
        "{CROSSING_LABEL}:{}{CROSSING_VALUE}",
        " ".repeat(LABEL_COLUMN - CROSSING_LABEL.len())
    );
    let redactor = Redactor::new(
        [SEED.to_owned(), crossing_seed.clone()],
        RedactionRules::common(),
    );
    assert_eq!(redactor.dropped_secrets(), 0, "both seeds are accepted");

    let facts = Facts {
        generated_at,
        title: "Example Support Bundle".to_owned(),
        redaction_statement: "Values below are redacted by the reporter.".to_owned(),
        sections: vec![
            Section::Facts {
                heading: "Build".to_owned(),
                facts: vec![
                    Fact::new("Version", "1.4.0"),
                    Fact::new("Channel", "stable"),
                    Fact::new("Note", "line one\nline two"),
                ],
            },
            Section::Facts {
                heading: "Workspace".to_owned(),
                facts: vec![Fact::new(CROSSING_LABEL, CROSSING_VALUE)],
            },
            Section::Items {
                heading: "Checks".to_owned(),
                items: vec!["network: ok".to_owned(), format!("token={SEED}")],
                empty: "no checks recorded".to_owned(),
            },
            Section::Items {
                heading: "Pending".to_owned(),
                items: Vec::new(),
                empty: "nothing pending".to_owned(),
            },
            Section::Records {
                heading: "Recent log".to_owned(),
                records: ring.tail(4),
                empty: "no records".to_owned(),
            },
            Section::Records {
                heading: "Earlier log".to_owned(),
                records: Vec::new(),
                empty: "no earlier records".to_owned(),
            },
        ],
    };

    let bundle = render(&redactor, &facts);
    assert_eq!(
        bundle.text, EXPECTED,
        "the contract's layout, byte for byte"
    );
    assert_eq!(
        bundle.generated_at, "2023-11-14T22:13:20.123Z",
        "the timestamp as it renders"
    );
    assert_eq!(
        bundle.redactions,
        bundle.text.matches(REDACTED).count(),
        "redactions is the marker count"
    );
    assert_eq!(bundle.redactions, 3, "three spans were redacted");
    assert!(
        bundle.text.contains("  [redacted]\n"),
        "the crossing span replaced its whole row"
    );
    assert!(!bundle.text.contains(CROSSING_VALUE), "the path is gone");
    for fragment in bundle.text.split(REDACTED) {
        assert!(
            !fragment.contains(SEED),
            "the seeded secret survived outside a marker"
        );
    }

    // Interior CR and LF never break the layout: each becomes the two
    // characters `\r` and `\n`, and a backslash is not escaped.
    assert_eq!(
        bundle.text.lines().count(),
        25,
        "one line per layout row, with no row broken by its value"
    );
    assert!(
        bundle.text.contains("line one\\nline two\n"),
        "an interior LF escapes instead of breaking the row"
    );

    // The same facts through an equivalent redactor render to the same bytes.
    let again = render(&redactor, &facts);
    assert_eq!(again.text, bundle.text, "render is deterministic");
    assert_eq!(again.redactions, bundle.redactions);

    format!(
        "exact {} line layout, {} redactions counted last, a span across a label and its value, CR/LF escaping, empty-section sentences, record rows and determinism",
        bundle.text.lines().count(),
        bundle.redactions
    )
}
