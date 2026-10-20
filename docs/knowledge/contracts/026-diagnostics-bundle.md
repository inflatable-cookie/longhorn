# 026 Diagnostics Bundle

Status: active compiled boundary
Owner: Longhorn maintainers
Created: 2026-10-19
Depends on: contracts 001, 003, 012, 024 and 025
Affects: `longhorn-diagnostics`, desktop support consumers

## Problem

Desktop support answers one question set per app: which build is installed,
what the setup steps say, which tools were found, whether the service is
connected, where the workspace is, what needs attention, and what the log
tail holds. Acowtancy Desktop renders that as one text in
`diagnostics/bundle.rs`, with every heading, label and empty-section
sentence compiled in, so the layout cannot move to Longhorn without taking
product text with it. Every Longhorn desktop app needs one tested layout;
none should grow its own.

Source analysis: row "`diagnostics::bundle`", section "Support and tooling"
of the Praxis map
[`shared-tooling-extraction.md`](https://github.com/inflatable-cookie/praxis/blob/dbad7b093f0452800c4df3a96c0798ebf788b08f/docs/knowledge/architecture/shared-tooling-extraction.md)
at `dbad7b0`. Donor: `acowtancy/acowtancy-v2` at `3c21b1ac8`,
`apps/bovine-desktop/src-tauri/src/diagnostics/bundle.rs` and its `tests.rs`.
From this contract on, this file owns the behaviour; the map and the donor
are evidence.

## Generic versus product

The crate is a pure renderer. It reads no filesystem, no network, no
credentials and no app state, and it owns no product text.

- Longhorn owns the layout: the row shapes, the label padding rule, the
  section separation, the final newline, the redact-last rule, and the
  marker count.
- The app owns every value and name: the title text, the redaction statement
  wording, the section list and order, the headings, the labels, the
  empty-section sentences, the generated-at instant, the facts themselves,
  the fact gatherers over live state, and the file name for a saved bundle.
- The app supplies the `longhorn_redact::Redactor` (contract 024) and so
  decides which secrets and rules apply.
- The app maps the returned `Bundle` into its own wire type. The crate has
  no serde, no ts-rs binding and no features.
- The timestamp comes from `longhorn_support_log::utc_timestamp` (contract
  025).

## Public API

Crate `longhorn-diagnostics`: depends on `longhorn-redact`,
`longhorn-support-log` and `std` only, no features, no TypeScript binding.

- `Fact { label, value }`, `Fact::new(label, value)`.
- `Section`: one of
  - `Facts { heading, facts }` — fact rows; an empty list renders the
    heading only;
  - `Items { heading, items, empty }` — list rows; when `items` is empty,
    `empty` renders as the only list row;
  - `Records { heading, records, empty }` — one row per
    `RingRecord::line()`; when `records` is empty, `empty` renders as the
    only list row.
- `Facts { generated_at: SystemTime, title, redaction_statement, sections }`.
- `Bundle { text, generated_at, redactions }`: `text` is the complete
  redacted text; `generated_at` is the timestamp as it renders in the text;
  `redactions` is the marker count.
- `render(&Redactor, &Facts) -> Bundle`.

## Layout

`render` is deterministic: the same facts through an equivalent redactor
render to the same bytes.

The text is, in order:

1. the title line;
2. `Generated: ` then the timestamp;
3. the redaction statement line;
4. one empty line;
5. the sections, each its heading line then its rows, with one empty line
   between consecutive sections and none after the last.

Row shapes:

- **Fact row:** two spaces, the label, `:`, then `18` minus the escaped
  label's byte length spaces (none when the label is at or over 18 bytes),
  then the value, then the newline. The padding counts bytes, not
  characters: a multi-byte label fills less of the column than it appears
  to.
- **List row:** `  - `, the item, the newline.
- **Record row:** two spaces, `RingRecord::line()`, the newline.
- **Empty sentence:** the sentence renders as a list row, dash included,
  for both `Items` and `Records`.

Lines: interior CR and LF never break the layout. In the title, the
redaction statement, every heading, label, value, list item and empty
sentence, each CR becomes the two characters `\r` and each LF the two
characters `\n`; a backslash is not escaped. This is contract 025's line
rule. Record rows are already escaped by `RingRecord::line()`.

Bounds: the crate applies no size bound. The inputs are already bounded by
their sources (contract 025). A consumer that wants a bound applies it to
the returned text, after redaction.

## Redact last

The crate builds the complete text first, then runs one
`Redactor::redact` call over all of it. A span that crosses a label, its
colon and its value is still caught, because no section, row or field is
redacted in isolation. The redactor repeats its internal passes per
contract 024.

**Order limitation:** the interior CR and LF of the caller's text are
escaped as the text is built, before the pass. The pass therefore sees the
two characters `\r` and `\n` where the caller's text had a break, and an
exact seed whose match depends on a raw line break is not caught. Shape
rules do not depend on the caller's breaks and still catch their spans — a
PEM block pasted into a value is one marker. The donor redacted the raw
text and caught such seeds; that is a listed delta, pinned by test. The
crate escapes after assembly because the escape must never move a caller's
marker: the marker count, the marker text and the caller's own
`[redacted]` inputs survive the pass untouched.

`redactions` is `Redaction::count`: the `[redacted]` occurrences in the
final text, including any the caller's inputs already carried. No public
path returns text that has not been redacted: `render` is the only
text-producing function, and `Bundle::text` is its redacted result.

## Compatibility for consumers

Against the Acowtancy donor at `3c21b1ac8`, `bundle.rs`:

- **Structure:** `DiagnosticsFact`, `DiagnosticsFacts`, `DiagnosticsBundle`
  and `render` become `Fact`, `Facts`, `Bundle` and
  `longhorn_diagnostics::render`. The fixed section list, the headings, the
  labels, the empty-section sentences and the `LOG_TAIL_LIMIT` heading text
  become caller values; Acowtancy keeps them.
- **Secrets:** `DiagnosticsFacts.secrets` is gone. The app passes a seeded
  `Redactor` instead of a secret list (contract 025's rule).
- **Wire:** `DiagnosticsBundle`'s serde derive, ts-rs binding, camelCase
  naming and `FILE_NAME` stay in Acowtancy. Longhorn's `Bundle` is not a
  wire type.
- **`Generated:` precision:** the donor formats seconds
  (`2026-10-21T12:00:00Z`); the crate takes `utc_timestamp`, which carries
  milliseconds (`2026-10-21T12:00:00.000Z`).
- **Interior CR and LF:** the donor wrote them raw; the crate escapes them
  per contract 025's line rule.
- **Raw-break seeds:** the donor redacted the raw text, so an exact seed
  spanning a raw CR or LF was caught; the crate escapes before the pass and
  is not (see Redact last). Pinned by test.
- **Padding:** the donor pads by the raw label's byte length; the crate
  pads by the escaped label's byte length. They are identical for labels
  without CR or LF, which is every label the donor produces.
- **Layout:** the row shapes, the section separation and the final newline
  are unchanged. With the same facts, the text is byte for byte the
  donor's, except for the deltas above.

## Required Evidence

- `effigy check:diagnostics` (QA group `diagnostics`): Clippy, rustdoc with
  warnings denied, the API-reference inventory, and `test:diagnostics`. The
  tests use the public API only. They run the donor tests (every support
  section carried, the plain empty-section sentences, seeded secrets never
  reaching the text and counted), then: a committed golden text rendered
  byte for byte, a secret split across a label and its value, seeded
  secrets in every section kind, a `RingRecord` tail, a label at or over
  the column, multi-byte UTF-8 labels, interior CR and LF, empty sections
  with their list-row sentences, the pinned raw-break-seed delta, a PEM
  block in a value caught as one marker, and a second render matching the
  first.

## Extraction state

Contract 003 steps 1–5 and 7 are done. Acowtancy Desktop migrated in
acowtancy-v2 PR 941 (merged 2026-10-20 at `8ed2e1219`) and took every Longhorn
crate at `v0.3.3`. Its `diagnostics/bundle.rs` renders through
`longhorn_diagnostics::render` with the app's seeded `Redactor`. Desktop keeps
the title, the section list, the labels, the `DiagnosticsBundle` wire type and
the save dialog. Longhorn's contract and tests are now authoritative.

Open:

- Step 6: second-consumer conformance has no named consumer yet.

## Stop Conditions

- A product text, name or wire shape is compiled into the crate.
- A public path returns text that has not been through the caller's
  redactor, or redacts anything smaller than the complete text.
- The layout stops being deterministic, or a byte-bound claim in this file
  no longer holds.
- The renderer grows a gatherer, a command, a dialog or any app-state read;
  those stay with the consumer.
