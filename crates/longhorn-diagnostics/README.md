# Longhorn Diagnostics

A pure diagnostics bundle renderer for desktop apps. Contract 026 owns its
behaviour.

The app gathers the facts — versions, setup steps, connection state, the
bounded log tail — and `render` lays them out as one support text in
sections, then redacts the whole text last with the app's
`longhorn_redact::Redactor`. A span that crosses a label and its value is
still caught. The returned `Bundle` carries the redacted text and how many
`[redacted]` markers it holds; no public path returns text that has not been
redacted.

The crate reads no filesystem, no network, no credentials and no app state,
and it owns no product text. The title, the redaction statement, the section
list and order, the headings, the labels and the empty-section sentences are
all caller values.

```rust
use std::time::SystemTime;

use longhorn_diagnostics::{Fact, Facts, Section, render};
use longhorn_redact::{RedactionRules, Redactor};

let redactor = Redactor::new(Vec::new(), RedactionRules::common());
let facts = Facts {
    generated_at: SystemTime::UNIX_EPOCH,
    title: "Support diagnostics".to_owned(),
    redaction_statement: "Secrets are removed.".to_owned(),
    sections: vec![Section::Facts {
        heading: "Application".to_owned(),
        facts: vec![Fact::new("Version", "1.2.3")],
    }],
};
let bundle = render(&redactor, &facts);
assert!(bundle.text.starts_with("Support diagnostics\nGenerated: 1970-01-01T00:00:00.000Z\n"));
assert_eq!(bundle.redactions, 0);
```
