# Card 170 Generated Projection Labels

Status: open
Owner: Tom
Updated: 2026-08-09
Governing refs: memo 022 (D1, D2); contract 013; contract 010
Depends on: Card 169

## Why

Label maps exist twice — once in `longhorn-poodle`, once in
`longhorn-poodle-svelte` — and only the Rust one fails to compile when a
variant is added. The TypeScript one returns `undefined` and renders as blank.
Thirteen restore compatibility arms, seven operation states, five
notification severities, and three enums the Svelte tier does not label at all
because it prints the serde wire form instead.

The operator decision on memo 022's D2 is **generate the TypeScript from
Rust**, extending the existing `longhorn-bindings` pipeline rather than
building new machinery.

## The complication found while sizing it

**Not every label can be a map.** `longhorn-bindings` emits static TypeScript
artifacts, and a static map only works for a data-free enum. Of the label sets
in scope:

| Label set | Variants carry data | Generatable as a map |
| --- | --- | --- |
| operation state | no | yes |
| notification severity | no | yes |
| restore integrity | no | yes |
| restore authenticity | no | yes |
| restore identity status | **yes** — `expected`, `archive` | no |
| restore compatibility | **yes** — 6 of 13 arms | no |

`"Migration required (3 → 7)"` is a function of the variant's fields, not a
constant, so it cannot be a lookup table. That is the thirteen-arm map — the
one with the worst drift risk and the one generation helps least.

Sizing this was worth doing before committing to a shape, and it means the
card is two pieces rather than one.

## Approach

1. **Generate the constant maps.** Data-free enums get a generated
   `Record<Variant, string>` per domain, emitted as a `longhorn-bindings`
   artifact and checked by `check:bindings`. Adding a variant then fails the
   gate rather than rendering blank.

2. **Generate a format table for the parameterised ones.** Each arm emits its
   field names and a format string with named placeholders; the TypeScript
   interpolates. Keeps one source of wording while letting each side
   interpolate in its own language.

   If that proves worse than the disease — the format strings will be doing
   real work and a bad abstraction here is worse than duplication — fall back
   to a **drift test**: a generated list of variant names that the TypeScript
   map must cover exhaustively, so a missing arm fails without moving the
   wording. Decide by trying (1) and (2) on one domain first.

3. **Delete the Svelte tier's serde-wire-form rendering** (memo 022, D1) as
   the maps land. `value={inspection.integrity}` becomes a labelled lookup.

## Do Not

- Move wording into the bindings crate. `longhorn-poodle` owns what a thing is
  called; the generator only carries it across.
- Generate into `packages/*` without a `check:` task. A generated artifact
  nobody verifies is a stale artifact with extra steps.

## Acceptance Criteria

- adding a variant to any enum in scope fails a gate rather than rendering
  blank in a webview
- the wording has exactly one source, or the exception is written down with
  its reason
- no Svelte surface renders a serde `rename_all` output as UI text

## Stop Conditions

- the format-table approach needs placeholder semantics rich enough to be a
  template language, in which case stop and take the drift test instead
- generation would require `longhorn-bindings` to depend on `poodle-specs`,
  which would drag a sibling-repository path dependency into the bindings
  gate. The label functions can be split from the spec-emitting ones if so.

## Notes

Poodle needs no change for any of this. The specs already carry everything;
this is entirely about who writes the words and how they cross the language
boundary.
