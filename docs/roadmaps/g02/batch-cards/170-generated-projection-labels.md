# Card 170 Generated Projection Labels

Status: complete
Completed: 2026-08-09
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

## What was built

The format table worked, so the drift-test fallback was not needed.

**Wording moved to the enums that own it.** `NotificationSeverity`,
`OperationStateProjection`, `RestoreIntegrityProjection` and
`RestoreAuthenticityProjection` each gained `ALL`, `wire_name` and `label`.
`longhorn-poodle` now delegates rather than restating. This also dissolved the
stop condition about `longhorn-bindings` needing `poodle-specs`: the generator
already depends on the domain crates, and a label was always a property of the
enum rather than of a Poodle projection. `InstallManager::label` was the
existing precedent.

**Templates, not strings, for the parameterised set.**
`RestoreDomainCompatibilityProjection::TEMPLATES` holds thirteen
`(wire name, template)` pairs. `render_label_template` fills `{name}`
placeholders, and the generated `renderLabelTemplate` mirrors it in
TypeScript. Both sides interpolate the *same* templates, so one source still
decides the wording — which was the whole worry.

One substitution rule, not a template language: a placeholder is a name in
braces and everything else is literal. An unknown placeholder is left as
written, so a mistake shows as `{typo}` on screen rather than a hole that
reads like deliberate wording. A test asserts no rendered label contains a
brace.

**Three generated artifacts**, written by the existing bindings pipeline and
checked by `check:bindings`:

- `packages/longhorn/src/notifications/generated/labels.ts`
- `packages/longhorn/src/operation/generated/labels.ts`
- `packages/longhorn/src/config/generated/labels.ts` and `label-template.ts`

Each map is typed `Record<Union, string>`, so a variant added to the union and
missing from the map is a TypeScript error at the point of use. A tagged
union's discriminant is not importable on its own, so the compatibility map is
keyed by indexing the union — `RestoreDomainCompatibilityProjection["status"]`.

**Three D1 sites fixed.** `RestoreSettingsPage.svelte` no longer renders
`inspection.integrity`, `inspection.authenticity` or an identity `status`
directly; `notificationStatusLabel` no longer returns
`record.draft.severity`. No Svelte surface renders a serde `rename_all`
output as UI text any more.

`identityLabel` is the one label written twice on purpose. Its mismatch arm
interpolates two fields into a sentence rather than a label, and a template
table for two arms is more machinery than it saves. Recorded rather than
hidden.

## Original approach

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

- [x] adding a variant to any enum in scope fails a gate rather than rendering
  blank in a webview — the map is `Record<Union, string>` and `check:bindings`
  compares the generated artifact
- [x] the wording has exactly one source, or the exception is written down
  with its reason — `identityLabel` is the one exception
- [x] no Svelte surface renders a serde `rename_all` output as UI text

## Stop Conditions

Neither fired.

- ~~the format-table approach needs placeholder semantics rich enough to be a
  template language~~ — one substitution rule was enough for all thirteen.
- ~~generation would require `longhorn-bindings` to depend on
  `poodle-specs`~~ — moving the wording onto the domain enums removed the
  question. The generator already depends on those crates.

## Notes

Poodle needs no change for any of this. The specs already carry everything;
this is entirely about who writes the words and how they cross the language
boundary.
