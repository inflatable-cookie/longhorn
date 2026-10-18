# 024 Text Redaction

Status: active compiled boundary
Owner: Longhorn maintainers
Created: 2026-10-18
Depends on: contracts 001, 003, and 012
Affects: `longhorn-redact`, desktop support consumers

## Problem

Desktop support text (diagnostics, install logs, Git stderr, CLI error
detail) carries secrets. Acowtancy Desktop holds three redactors that
disagree on the marker, the rules and the boundaries. Every Longhorn desktop
app needs one tested engine; none should grow its own.

Source analysis: section "First unit: secret redaction" of the Praxis map
[`shared-tooling-extraction.md`](https://github.com/inflatable-cookie/praxis/blob/dbad7b093f0452800c4df3a96c0798ebf788b08f/docs/knowledge/architecture/shared-tooling-extraction.md)
at `dbad7b0`. Donor: `acowtancy/acowtancy-v2` at `1d2d13022`,
`apps/bovine-desktop/src-tauri/src/diagnostics/redact.rs` and
`managed_tools/install_log.rs`. From this contract on, this file owns the
behaviour; the map and the donor are evidence.

## Generic versus product

- Longhorn owns the engine and the `RedactionRules::common()` vocabulary of
  published vendor formats.
- A product decides which secrets to seed, which rules to enable, and where
  output is redacted.
- Product names (environment variables, config keys) enter through
  `assignment_keys` or as seeded secrets. They are never compiled into
  Longhorn.

## Public API

Crate `longhorn-redact`: `std` only, no features, no TypeScript binding.

- `REDACTED`: the one marker, `[redacted]`.
- `RedactionRules { token_prefixes, assignment_keys, bearer, url_userinfo,
  pem_private_keys, jwt }`. `common()` turns every rule on with the GitHub,
  npm, Anthropic, OpenAI, Slack, Google and AWS prefixes and the donor's
  credential assignment keys. `exact_only()` turns every shape rule off.
- `Redactor::new(secrets, rules)`: secrets are trimmed and deduplicated. A
  trimmed value under four characters is not accepted.
  `dropped_secrets()` reports how many supplied values were not accepted,
  never the values. A caller may refuse to run when it is non-zero.
- `Redactor::redact(text) -> Redaction { text, count, replaced }`. `count` is
  the `[redacted]` occurrences in `text`, including any in the input.
  `replaced` is the markers this call inserted, one per merged span, summed
  over passes.

## Span model

Rules never run on each other's output within a pass, so rule order does not
matter.

1. Protected spans are the `[redacted]` occurrences in the pass's input.
2. Every enabled rule finds candidate spans in the pass's input. A candidate
   is clipped around protected spans: the markers stay, and each non-empty
   run outside them becomes its own candidate. A candidate wholly inside
   markers leaves nothing. A marker never shields the text around it.
3. Clipped candidates from all rules are unioned. Overlapping or touching
   candidates merge into one span; a protected marker always separates
   them. Each merged span becomes one marker.
4. Passes repeat until one finds no candidate. Every pass replaces at least
   one character outside markers and never alters a marker, so this ends,
   and `redact` is idempotent.

In boundary tests a marker counts as a boundary character, never a token
character. A token exposed next to a marker by one pass is caught by the
next pass.

## Rules

Each rule only proposes spans.

- **Exact secrets:** every occurrence of an accepted secret, case-sensitive,
  overlapping occurrences included. A secret inside the marker text, such as
  `redacted`, still matches outside markers.
- **PEM private keys:** from the start of a line holding `-----BEGIN ` then
  `PRIVATE KEY` through the end of the next line holding `-----END ` then
  `PRIVATE KEY`. Line terminators (`\n`, `\r\n`) stay, so the marker sits on
  its own line. Both markers on one line redact that line. A block with no
  END line runs to the end of the text. Public-key and certificate blocks
  are kept.
- **URL userinfo:** in `scheme://authority`, the authority runs to the next
  `/`, `?`, `#` or whitespace. The userinfo, up to the last `@`, is proposed
  only when it holds `:`. `https://git@host` is kept; a token used as a user
  name is caught by the prefix rule.
- **Bearer:** after a case-insensitive `Bearer` and one space, the credential
  up to whitespace, a quote or a comma. The scheme word stays. A credential
  under four characters, measured before clipping, is not proposed, so
  "bearer of" is prose.
- **Assignments:** each key matches case-insensitively where the preceding
  character is not ASCII alphanumeric (`_` is a boundary, so
  `NODE_AUTH_TOKEN` matches `token`), then optional spaces or tabs, then `=`
  or `:`, then optional spaces or tabs. The value:
  - quoted: up to the closing quote, or the line end when it does not close;
  - unquoted: up to whitespace, `&`, `,` or `;`;
  - for the key `authorization`: a value starting `Bearer ` (any case)
    proposes nothing, because the Bearer rule owns it. Any other value runs
    to the line end.
  - Keys inside a URL authority are ignored; the userinfo rule owns that
    region.
- **JWT:** a run starting `eyJ` up to whitespace that holds at least two
  dots.
- **Token prefixes:** a listed prefix, case-sensitive, where the preceding
  character is not `[A-Za-z0-9_-]`, then at least one body character. The
  body runs while characters are `[A-Za-z0-9_-]`; `@`, `/` and `.` end it.
  A bare prefix is kept. AWS key IDs use their issued upper-case
  `AKIA`/`ASIA`; Google API keys use `AIza`.

Empty keys and empty prefixes propose nothing.

## Guarantees

The conformance test checks each one for every case:

- An accepted secret appears in the output only where the occurrence
  overlaps a marker. The proof splits the text on markers and searches each
  fragment. Dropped seeds are not protected.
- Idempotent: a second `redact` returns the same `text` and `count`, with
  `replaced == 0`.
- `count` equals the marker occurrences in `text`, and `count >= replaced`.
- No panic on any UTF-8 input. Characters outside merged spans are
  unchanged.

## Expected outputs

`fixtures/redact/cases.json` is the committed case table: input, seeds, rule
set, expected text, `count`, `replaced` and `dropped_secrets`. It holds every
row of the map's expected-output table, the cross-rule rows (seed inside a
prefix or userinfo span, a token exposed next to a marker, marker collision,
values that already hold a marker), and boundary cases: overlapping seeds, a
seed next to multi-byte UTF-8, adjacent seeds that merge, CRLF text, `Bearer`
at the end of input, and quoted values. A changed expected output is a
contract change.

Rows that fix the composition:

| Input | Output | Why |
| --- | --- | --- |
| `header Authorization: Bearer abc.def.ghi, tail` | `header Authorization: Bearer [redacted], tail` | Bearer rule; `authorization` proposes nothing |
| `token=ghp_12345` | `token=[redacted]`, `replaced` 1 | Assignment and prefix spans merge |
| `abxyghp_abc`, seed `abxy` | `[redacted][redacted]`, `replaced` 2 | Pass 2 catches the prefix exposed by pass 1 |
| `[redacted] and redacted`, seed `redacted` | `[redacted] and [redacted]`, `count` 2, `replaced` 1 | The occurrence inside the marker clips to nothing |
| `password=left[redacted]right` | `password=[redacted][redacted][redacted]`, `replaced` 2 | The marker separates the clipped runs |
| `Authorization: Bearer` | `Authorization: [redacted]` | No space after `Bearer`, so the `authorization` value runs to the line end |

## Compatibility for consumers

Against the Acowtancy donor at `1d2d13022`:

- One marker, `[redacted]`. `install_log` wrote `[REDACTED]` and
  `[REDACTED PRIVATE KEY]`.
- `authorization` values with a `Bearer` scheme keep the scheme word and the
  text after the credential. `install_log` consumed the rest of the line.
- Prefixes match case-sensitively at a token boundary. `install_log` matched
  `GHP_…` and `asia…` anywhere, so "Asian" became a marker.
- A prefix body stops at any character outside `[A-Za-z0-9_-]`. `install_log`
  ran to whitespace or one of `"`, `'`, `)`, `]`, `,`, `;`, so `ya29.a0b.c` now keeps `.c`.
- Userinfo without a password is kept. `install_log` redacted any userinfo.
- `Bearer` credentials under four characters are kept by both former paths.
- `Bearer` matches in any case. `diagnostics::redact` matched `Bearer ` only.
- JWTs need a case-sensitive `eyJ`; PEM markers are case-sensitive.
  `install_log` matched both in any case.
- The four-character secret floor counts characters, not bytes. A two- or
  three-character non-ASCII secret is now dropped and reported.
- `diagnostics::redact` gains assignments, JWTs and the non-GitHub prefixes.
  `git::redact_git_detail` gains every shape rule once it seeds a `Redactor`
  with `common()`.

The map checked every Acowtancy redactor assertion against these rules.
Only the `[REDACTED PRIVATE KEY]` presence assertion in `install_log.rs`
changes, to `[redacted]`.

## Required Evidence

- `effigy check:redact` (QA group `redact`): Clippy, rustdoc with warnings
  denied, the API-reference inventory, and `test:redact`. That test runs
  the case table and the guarantees through the public API, the donor's
  redactor tests with neutral values, and a deterministic generated-UTF-8
  sweep.

## Extraction state

Contract 003 steps 1–4 are done: donor named, donor tests captured, contract
defined, crate implemented. Open:

- Acowtancy migration (step 5) needs a Longhorn release and runs in
  Acowtancy's own Queue.
- Second-consumer conformance (step 6) has no named consumer yet.
- Donor copies stay authoritative in Acowtancy until its cutover.

## Stop Conditions

- A product wants its own names in `common()`.
- A rule needs to read another rule's output within a pass.
- A change would make `redact` non-idempotent or let `count < replaced`.
