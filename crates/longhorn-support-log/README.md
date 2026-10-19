# Longhorn Support Log

Bounded, redacted support logs for desktop apps. Contract 025 owns their
behaviour.

- `RingLog`: a fixed-capacity in-memory log, oldest first. Lines read
  `2026-10-01T12:00:00.000Z [error] github: detail`.
- `RotatingLog`: a size-bounded line log on disk with numbered rotations.
  It writes `<timestamp> <event> <detail>` lines or `JsonRecord` lines.
  A discarding log writes nowhere.
- `bound_field` and `bounded_tail`: bound one text field, or keep the newest
  lines and bytes of process output behind `[earlier output omitted]`.

Every text goes through the caller's `longhorn_redact::Redactor` before any
cut, so a cut never exposes part of a secret. Logging never panics and never
fails its caller; a `RotatingLog` counts what it drops in `stats()`.

The app owns every product choice: capacities, bounds, the directory, file
names, record fields, and any process-wide instance.

```rust
use longhorn_redact::{RedactionRules, Redactor};
use longhorn_support_log::{TailBounds, bounded_tail};

let redactor = Redactor::new(Vec::new(), RedactionRules::common());
let output = "step 1\nstep 2\ntoken=ghp_secret";
let tail = bounded_tail(&redactor, output, TailBounds { lines: 2, bytes: 1024 });
assert_eq!(tail, "step 2\ntoken=[redacted]");
```
