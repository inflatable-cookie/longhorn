//! Contract 024 (`longhorn-redact`): the expected-output shapes and the
//! guarantees, replayed through the public API with neutral values.
//!
//! The rows below are contract 024's composition table and its boundary
//! neighbours. They are redeclared here rather than read from the repository's
//! case table because this consumer sits outside the workspace: a conformance
//! consumer states what the contract promises, and a changed promise shows up
//! as a failed assertion in both places.

use longhorn_redact::{REDACTED, RedactionRules, Redactor};

/// One contract row: seeds, input, the promised text, and both counts.
struct Case {
    name: &'static str,
    seeds: &'static [&'static str],
    input: &'static str,
    expected: &'static str,
    count: usize,
    replaced: usize,
}

const CASES: &[Case] = &[
    Case {
        name: "bearer-keeps-scheme-and-tail",
        seeds: &[],
        input: "header Authorization: Bearer abc.def.ghi, tail",
        expected: "header Authorization: Bearer [redacted], tail",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "authorization-value-owns-the-line",
        seeds: &[],
        input: "authorization: private-value next",
        expected: "authorization: [redacted]",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "authorization-bearer-word-without-space",
        seeds: &[],
        input: "Authorization: Bearer",
        expected: "Authorization: [redacted]",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "assignment-and-prefix-merge",
        seeds: &[],
        input: "token=ghp_12345",
        expected: "token=[redacted]",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "token-exposed-next-to-marker",
        seeds: &["abxy"],
        input: "abxyghp_abc",
        expected: "[redacted][redacted]",
        count: 2,
        replaced: 2,
    },
    Case {
        name: "marker-collision",
        seeds: &["redacted"],
        input: "[redacted] and redacted",
        expected: "[redacted] and [redacted]",
        count: 2,
        replaced: 1,
    },
    Case {
        name: "assignment-value-split-by-marker",
        seeds: &[],
        input: "password=left[redacted]right",
        expected: "password=[redacted][redacted][redacted]",
        count: 3,
        replaced: 2,
    },
    Case {
        name: "url-userinfo-absorbs-a-token-user-name",
        seeds: &[],
        input: "https://x-access-token:gho_secret0123456789@example.com/team/repo.git",
        expected: "https://[redacted]@example.com/team/repo.git",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "pem-private-key-block",
        seeds: &[],
        input: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXk=\n-----END OPENSSH PRIVATE KEY-----",
        expected: "[redacted]",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "jwt-needs-two-dots",
        seeds: &[],
        input: "id eyJhbGciOi.eyJzdWIi.c2lnbmF0dXJl end",
        expected: "id [redacted] end",
        count: 1,
        replaced: 1,
    },
    Case {
        name: "exact-seed-only",
        seeds: &["s3cr3t-passphrase"],
        input: "the passphrase is s3cr3t-passphrase now",
        expected: "the passphrase is [redacted] now",
        count: 1,
        replaced: 1,
    },
];

pub(crate) fn run() -> String {
    let rules = RedactionRules::common();
    for case in CASES {
        let redactor = Redactor::new(
            case.seeds.iter().map(|seed| (*seed).to_owned()),
            rules.clone(),
        );
        assert_eq!(
            redactor.dropped_secrets(),
            0,
            "{}: a case seed was below the four-character floor",
            case.name
        );
        let redaction = redactor.redact(case.input);
        assert_eq!(
            redaction.text, case.expected,
            "{}: redacted text",
            case.name
        );
        assert_eq!(redaction.count, case.count, "{}: marker count", case.name);
        assert_eq!(
            redaction.replaced, case.replaced,
            "{}: inserted markers",
            case.name
        );

        // Every guarantee contract 024 states, checked on this case.
        let repeat = redactor.redact(&redaction.text);
        assert_eq!(
            repeat.text, redaction.text,
            "{}: idempotent text",
            case.name
        );
        assert_eq!(
            repeat.count, redaction.count,
            "{}: idempotent count",
            case.name
        );
        assert_eq!(repeat.replaced, 0, "{}: idempotent insertions", case.name);
        assert_eq!(
            redaction.count,
            redaction.text.matches(REDACTED).count(),
            "{}: count is the marker occurrences in the text",
            case.name
        );
        assert!(
            redaction.count >= redaction.replaced,
            "{}: count >= replaced",
            case.name
        );
        for seed in case.seeds {
            for fragment in redaction.text.split(REDACTED) {
                assert!(
                    !fragment.contains(seed),
                    "{}: a seeded secret survived outside a marker",
                    case.name
                );
            }
        }
    }

    // `exact_only()` turns every shape rule off; the shape that `common()`
    // redacts stays untouched, and a seeded secret is still caught.
    let exact_only = Redactor::new(
        ["s3cr3t-passphrase".to_owned()],
        RedactionRules::exact_only(),
    );
    let untouched = exact_only.redact("token=ghp_12345 and s3cr3t-passphrase");
    assert_eq!(
        untouched.text, "token=ghp_12345 and [redacted]",
        "exact_only"
    );
    assert_eq!(untouched.replaced, 1, "exact_only inserts one marker");

    // The four-character floor counts characters and reports drops, never
    // values. An accepted secret is case-sensitive.
    let floor = Redactor::new(
        ["ab".to_owned(), String::new(), "日本語".to_owned()],
        RedactionRules::common(),
    );
    assert_eq!(floor.dropped_secrets(), 3, "short and non-ASCII seeds drop");
    assert_eq!(
        floor.redact("a table of contents 日本語").text,
        "a table of contents 日本語",
        "dropped seeds protect nothing"
    );
    let seeded = Redactor::new(
        ["seeded-product-credential".to_owned()],
        RedactionRules::common(),
    );
    assert_eq!(
        seeded.redact("SEEDED-PRODUCT-CREDENTIAL").text,
        "SEEDED-PRODUCT-CREDENTIAL",
        "secret matching is case-sensitive"
    );

    // A multi-byte input around a span is cut on character boundaries, not
    // bytes, and never panics.
    let unicode = seeded.redact("日本語 seeded-product-credential 日本語");
    assert_eq!(unicode.text, "日本語 [redacted] 日本語", "multi-byte text");
    assert_eq!(unicode.count, 1, "multi-byte marker count");

    // A public-key block is not a private key and is kept whole.
    let public = "-----BEGIN PUBLIC KEY-----\nbody-line\n-----END PUBLIC KEY-----";
    assert_eq!(
        seeded.redact(public).text,
        public,
        "public-key blocks are kept"
    );

    format!(
        "{} contract rows with every guarantee, plus exact_only, the seed floor, case sensitivity and multi-byte boundaries",
        CASES.len()
    )
}
