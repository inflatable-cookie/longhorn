# Longhorn Redact

One std-only secret-redaction engine for desktop support text: diagnostics,
install logs, Git stderr and CLI error detail. Contract 024 owns its behaviour.

A `Redactor` holds exact secrets the app knows and a `RedactionRules` set of
shape rules (token prefixes, credential assignments, `Bearer` credentials,
URL userinfo, PEM private keys and JWTs). `redact` replaces every matched span
with the one marker `[redacted]` and reports how many markers the text holds
and how many this call inserted.

The crate owns the engine and the `RedactionRules::common()` vocabulary of
published vendor formats. A product decides which secrets to seed, which
rules to enable, and where output is redacted. Product-specific names go in
through `assignment_keys` or as seeded secrets.

```rust
use longhorn_redact::{RedactionRules, Redactor};

let redactor = Redactor::new(vec!["s3cr3t-value".to_owned()], RedactionRules::common());
let redaction = redactor.redact("push failed with s3cr3t-value");
assert_eq!(redaction.text, "push failed with [redacted]");
assert_eq!(redaction.replaced, 1);
```
