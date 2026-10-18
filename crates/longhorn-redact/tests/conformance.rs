//! Contract 024 conformance: the committed case table, the four guarantees,
//! the donor redactor tests, and a no-panic sweep over generated UTF-8.
//! Public API only.

use longhorn_redact::{REDACTED, Redaction, RedactionRules, Redactor};
use serde_json::Value;

const CASES: &str = include_str!("../../../fixtures/redact/cases.json");

fn field<'a>(case: &'a Value, name: &str) -> &'a Value {
    case.get(name)
        .unwrap_or_else(|| panic!("case is missing `{name}`: {case}"))
}

fn text(case: &Value, name: &str) -> String {
    field(case, name)
        .as_str()
        .unwrap_or_else(|| panic!("`{name}` is not a string"))
        .to_owned()
}

fn number(case: &Value, name: &str) -> usize {
    let value = field(case, name)
        .as_u64()
        .unwrap_or_else(|| panic!("`{name}` is not a number"));
    usize::try_from(value).expect("count fits usize")
}

fn rules(name: &str) -> RedactionRules {
    match name {
        "common" => RedactionRules::common(),
        "exact_only" => RedactionRules::exact_only(),
        other => panic!("unknown rule set `{other}`"),
    }
}

/// The seeds `Redactor::new` accepts, by the contract's own rule.
fn accepted(seeds: &[String]) -> Vec<String> {
    seeds
        .iter()
        .map(|seed| seed.trim().to_owned())
        .filter(|seed| seed.chars().count() >= 4)
        .collect()
}

/// Checks the guarantees every redaction must hold.
fn assert_guarantees(label: &str, redactor: &Redactor, seeds: &[String], first: &Redaction) {
    for fragment in first.text.split(REDACTED) {
        for seed in seeds {
            assert!(
                !fragment.contains(seed.as_str()),
                "{label}: accepted seed survives outside markers in {:?}",
                first.text
            );
        }
    }
    let second = redactor.redact(&first.text);
    assert_eq!(second.text, first.text, "{label}: second pass changed text");
    assert_eq!(second.count, first.count, "{label}: second pass changed count");
    assert_eq!(second.replaced, 0, "{label}: second pass replaced spans");
    assert_eq!(
        first.count,
        first.text.matches(REDACTED).count(),
        "{label}: count is not the marker occurrences"
    );
    assert!(first.count >= first.replaced, "{label}: count < replaced");
}

#[test]
fn case_table_holds_with_guarantees() {
    let table: Value = serde_json::from_str(CASES).expect("cases.json parses");
    assert_eq!(text(&table, "schema"), "longhorn.redact-cases.v1");
    let cases = field(&table, "cases").as_array().expect("cases array");
    assert!(!cases.is_empty());
    for case in cases {
        let name = text(case, "name");
        let seeds = field(case, "seeds")
            .as_array()
            .expect("seeds array")
            .iter()
            .map(|seed| seed.as_str().expect("seed string").to_owned())
            .collect::<Vec<_>>();
        let redactor = Redactor::new(seeds.clone(), rules(&text(case, "rules")));
        let result = redactor.redact(&text(case, "input"));

        assert_eq!(result.text, text(case, "expected"), "{name}: text");
        assert_eq!(result.count, number(case, "count"), "{name}: count");
        assert_eq!(result.replaced, number(case, "replaced"), "{name}: replaced");
        assert_eq!(
            redactor.dropped_secrets(),
            number(case, "dropped_secrets"),
            "{name}: dropped_secrets"
        );
        assert_guarantees(&name, &redactor, &accepted(&seeds), &result);
    }
}

// Ported from the donor's diagnostics and install-log redactor tests, with
// neutral hosts and tokens. Token shapes are built from parts so the source
// holds no credential-shaped literal.

#[test]
fn scrubs_github_tokens_bearer_credentials_urls_and_private_keys() {
    let redactor = Redactor::new(Vec::new(), RedactionRules::common());
    let classic = ["ghp_", "ABCdef0123456789XYZ"].concat();
    let fine_grained = ["github_pat_", "11ABCDEFG0abcdefghijklmnop"].concat();
    let oauth = ["gho_", "secret0123456789"].concat();
    let begin = ["-----BEGIN OPENSSH ", "PRIVATE KEY-----"].concat();
    let end = ["-----END OPENSSH ", "PRIVATE KEY-----"].concat();
    let text = format!(
        "token {classic} done\n\
         pat {fine_grained}\n\
         header Authorization: Bearer abc.def.ghi, tail\n\
         remote https://x-access-token:{oauth}@example.invalid/org/repo.git\n\
         {begin}\nb3BlbnNzaC1rZXk=\n{end}\n"
    );
    let redaction = redactor.redact(&text);
    let result = &redaction.text;

    assert!(!result.contains(&classic));
    assert!(!result.contains(&fine_grained));
    assert!(!result.contains("abc.def.ghi"));
    assert!(!result.contains(&oauth));
    assert!(!result.contains("b3BlbnNzaC1rZXk="));
    assert!(!result.contains("-----BEGIN"));
    assert!(result.contains("token [redacted] done"));
    assert!(result.contains("Bearer [redacted], tail"));
    assert!(result.contains("https://[redacted]@example.invalid/org/repo.git"));
    assert!(redaction.count >= 5);
    assert_guarantees("donor-scrub", &redactor, &[], &redaction);
}

#[test]
fn leaves_ordinary_github_material_alone() {
    let redactor = Redactor::new(Vec::new(), RedactionRules::common());
    let text = "origin is github.com/org/repo and https://github.com/login/device";
    assert_eq!(redactor.redact(text).text, text);
}

#[test]
fn ignores_trivially_short_secrets() {
    let redactor = Redactor::new(vec!["ab".to_owned(), String::new()], RedactionRules::common());
    assert_eq!(redactor.dropped_secrets(), 2);
    assert_eq!(
        redactor.redact("a table of contents").text,
        "a table of contents"
    );
}

#[test]
fn redacts_common_token_forms_and_url_credentials() {
    let redactor = Redactor::new(Vec::new(), RedactionRules::common());
    let npm = ["npm_", "secret"].concat();
    let openai = ["sk-proj-", "secret"].concat();
    let begin = ["-----BEGIN ", "PRIVATE KEY-----"].concat();
    let end = ["-----END ", "PRIVATE KEY-----"].concat();
    let text = format!(
        "Authorization: Bearer abcdef\n\
         token=ghp_12345 NODE_AUTH_TOKEN={npm} AWS_SECRET_ACCESS_KEY=aws_secret \
         OPENAI_API_KEY={openai} https://user:password@example.invalid/path\n\
         {begin}\nprivate-key-value\n{end}"
    );
    let redaction = redactor.redact(&text);
    let result = &redaction.text;

    assert!(!result.contains("abcdef"));
    assert!(!result.contains("ghp_12345"));
    assert!(!result.contains(&npm));
    assert!(!result.contains("aws_secret"));
    assert!(!result.contains(&openai));
    assert!(!result.contains("user:password"));
    assert!(!result.contains("private-key-value"));
    // The donor wrote `[REDACTED PRIVATE KEY]`; contract 024 has one marker,
    // on its own line.
    assert!(result.ends_with("\n[redacted]"));
    assert_guarantees("donor-install-log", &redactor, &[], &redaction);
}

/// A small deterministic generator: xorshift over fragments that exercise
/// every rule, markers, line endings and multi-byte characters.
struct Fragments(u64);

impl Fragments {
    const PIECES: &'static [&'static str] = &[
        "[redacted]",
        "redacted",
        "[redact",
        "ed]",
        "ghp_",
        "AKIA",
        "sk-ant-",
        "ya29.",
        "abc",
        "XYZ09",
        "_",
        "-",
        ".",
        "eyJ",
        "Bearer ",
        "bearer",
        "Authorization",
        "token",
        "password",
        "=",
        ": ",
        "\"",
        "'",
        "&",
        ",",
        ";",
        "://",
        "https://",
        "@",
        "/",
        "?",
        "#",
        " ",
        "\t",
        "\n",
        "\r\n",
        "\r",
        "-----BEGIN PRIVATE KEY-----",
        "-----END PRIVATE KEY-----",
        "-----BEGIN ",
        "é",
        "漢",
        "😀",
        "\u{0}",
        "s3cr3t",
    ];

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).expect("bound fits u64"))
            .expect("index fits usize")
    }

    fn text(&mut self, max_pieces: usize) -> String {
        let pieces = self.below(max_pieces + 1);
        (0..pieces)
            .map(|_| Self::PIECES[self.below(Self::PIECES.len())])
            .collect()
    }
}

#[test]
fn never_panics_and_holds_guarantees_on_generated_utf8() {
    let mut generator = Fragments(0x9e37_79b9_7f4a_7c15);
    for round in 0..4000 {
        let seeds = (0..generator.below(4))
            .map(|_| generator.text(3))
            .collect::<Vec<_>>();
        let rules = if round % 5 == 0 {
            RedactionRules::exact_only()
        } else {
            RedactionRules::common()
        };
        let redactor = Redactor::new(seeds.clone(), rules);
        let input = generator.text(24);
        let result = redactor.redact(&input);
        assert_guarantees(
            &format!("round {round}: {input:?} seeds {seeds:?}"),
            &redactor,
            &accepted(&seeds),
            &result,
        );
    }
}
