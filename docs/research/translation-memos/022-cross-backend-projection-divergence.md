# 022 Cross-backend Projection Divergence

Status: decided 2026-08-09 — eight implemented, one open
Owner: Tom
Updated: 2026-08-09
Depends on: memo 021; contract 013; contract 020; Card 169

## Prompt

Card 169 built `longhorn-poodle`, projecting six Longhorn domains into
`poodle-specs`, and drew four of them in a real GPUI window. It closed on an
admission: nobody has put the Svelte and GPUI renderings of the same facts
side by side, so "the two backends agree" was left as an assumption.

It is not a safe one. Building the second implementation surfaced divergences
that were invisible while there was only one, and most of them are not bugs in
either tier — they are places where the same fact can honestly be shown two
ways and nobody has yet decided which. That is an operator call, not an
engineering one, so this memo states each as a question with options and a
recommendation rather than picking silently.

## Sources

Read directly, 2026-08-09: every `poodle/projectors.ts` and pure model file in
`packages/longhorn-poodle-svelte`; `packages/longhorn/src/settings/registry.ts`;
the six modules of `crates/longhorn-poodle`; `poodle-specs`, `poodle-render`
and `poodle-gpui-node-backend`. One GPUI window drawn and inspected
(`prototypes/gpui-windowing/src/bin/render.rs`).

## What is genuinely settled

Worth saying first, because the list below is long and could read as though
the tiers disagree everywhere. They mostly do not.

- **Notification severity collapse agrees.** Svelte's
  `notificationStatusTone` maps `error` and `critical` to `danger`. Rust's
  `tone_for` does the same. Independently written, same answer.
- **Operation state labels agree.** Svelte's `stateLabels` and Rust's
  `state_label` produce identical strings for all seven states, and identical
  tones for all seven — including `cancelled` to neutral, which is the one
  that looks wrong at a glance.
- **Cancel eligibility agrees.** Both require `supported` *and* a state of
  `queued` or `running`.
- **Restore compatibility labels agree** for all thirteen classifications.

So the divergences below are specific, not systemic.

## The divergences

### D1. Serde wire form is rendered as UI text

**What.** Three places show a serde `rename_all = "camelCase"` output directly
to an operator:

- `RestoreSettingsPage.svelte` — `value={inspection.integrity}` and
  `value={inspection.authenticity}` render `verified`, `unauthenticated`.
- The same page renders `inspection.identity.application.status` as
  `compatible` / `mismatch`.
- `notificationStatusLabel` returns `` `${record.draft.severity}` `` —
  `critical`, lowercase.

Rust holds the enum and cannot do this, so `longhorn-poodle` writes
`"Verified"`, `"Unauthenticated"`, `"Mismatch: host expects X, archive
declares Y"`.

**Why it matters beyond appearance.** Renaming a serde variant silently
changes what a webview displays and changes nothing in Rust. The wire format
is load-bearing for the UI on one backend and not the other.

**Options.**
1. **Rust is right; fix Svelte.** Add label functions to the Svelte
   projectors mirroring the Rust ones. Cost: writing them, plus the
   duplication in D2.
2. Accept the difference as cosmetic and let the backends read differently.
3. Make Rust emit the wire form too, so they match. Rejected on sight: it
   makes the defect the standard.

**Recommendation: 1.** An operator reading `unauthenticated` in a restore
dialog is being shown an implementation detail at the moment they most need
plain language.

### D2. Label maps exist twice and only one fails to compile

**What.** `compatibility_label` (thirteen arms) exists in Rust and in
`restore-model.ts`. Operation state labels exist in both. Any new variant
means editing two maps, and only the Rust one is a compile error if missed.
The Svelte one silently returns `undefined`.

**Options.**
1. **Generate the TypeScript map from Rust**, alongside the existing `ts-rs`
   bindings. The variants already cross that way; the labels would follow.
2. Add a test that fails when the two maps disagree, without generating.
3. Accept the drift risk and rely on review.

**Recommendation: 1**, with 2 as the cheap interim. The bindings pipeline
already exists, so this is an extension rather than new machinery.

### D3. A projection rule lives in the client tier, in TypeScript only

**What.** `projectSettingsRegistry` in
`packages/longhorn/src/settings/registry.ts` states a Longhorn rule — drop
sections with no pages, then drop modules left with no sections. It is a pure
function over Rust-generated types, in the *client* tier rather than the
projection tier, and Rust had no statement of it until `longhorn-poodle::
settings::navigation` was written.

`searchSettingsRegistry` and `resolveSettingsDeepLink` are in the same file
and still have no Rust counterpart.

**Options.**
1. **Move all three rules to Rust** and have the TypeScript call across or be
   generated from it.
2. Port them to Rust and keep both, accepting D2's drift risk for three more
   functions.
3. Leave search and deep-link resolution in TypeScript, and accept that a
   GPUI settings surface has no search.

**Recommendation: 2 now, 1 later.** A settings surface without search is not
shippable, and option 1 depends on D2's decision.

### D4. Search normalisation is locale-pinned on one side only

**What.** `searchSettingsRegistry` normalises with
`toLocaleLowerCase("en-US")`. Rust's `str::to_lowercase` is Unicode-default
and locale-free. They disagree on Turkish dotless i, and on any future locale
the product ships in.

**Options.**
1. **Rust is canonical; change the TypeScript to plain `toLowerCase()`.**
2. Pin Rust to en-US by adding a locale-aware dependency.
3. Make normalisation a declared part of the settings contract, so neither
   tier chooses independently.

**Recommendation: 3, implemented as 1.** Search matching is a contract
behaviour, and it should be written down rather than inherited from whichever
standard library was reached for.

### D5. Critical and Error are indistinguishable on screen

**What.** Confirmed by looking, not by test. In the GPUI window the "Sync
failed" (`Error`) and "Storage is read-only" (`Critical`) toasts are the same
tint and weight; only their words differ. Longhorn has five severities and
Poodle has four tones. `ToneMapping::is_lossy` reports the collapse; the
Svelte tier flattens it silently and reports nothing.

**Options.**
1. **Poodle gains a fifth tone.** Cleanest, and the only option that makes the
   distinction visible without every surface reimplementing it. Cost: a Poodle
   change affecting every component that takes a `StatusTone`, and a token.
2. **Longhorn carries the distinction in text** — a `Critical` toast titled
   with a prefix, or a persistent banner rather than a toast. No Poodle
   change.
3. **Longhorn drops to four severities.** Honest if `Critical` and `Error`
   never actually differ in what an operator should do — but they do:
   `Critical` in the ledger means the application cannot continue safely.
4. Accept the collapse and rely on wording.

**Recommendation: 2 first, 1 if it recurs.** Option 1 is the right shape and
the wrong cost for one caller; if a second domain needs a fifth level, it
becomes right on both counts.

### D6. The Svelte tier re-declares Poodle specs as prop bundles

**What.** `OperationStatusTone` re-declares Poodle's `StatusTone` member for
member. `OperationProgressView` re-declares `ProgressSpec` field for field.
Neither exists because Poodle lacked something — a Svelte projection hands
props to a component and needs a prop-shaped object, while a Rust projection
returns the spec itself.

**Why it matters.** Every re-declared shape is a place the backends drift
while both still compile. Poodle adding a `StatusTone` member does not reach
the Svelte projector.

**Options.**
1. **Import the spec types from `@inflatable-cookie/poodle-svelte`** instead
   of re-declaring, wherever a matching type is exported.
2. Generate the TypeScript spec types from `poodle-specs`, as D2 proposes for
   labels.
3. Accept it as the cost of the prop model.

**Recommendation: 1** where the export exists, 2 where it does not. This is
Poodle-side work in both cases and should be raised there.

### D7. Read state reaches one backend and not the other

**What.** `notificationStatusLabel` appends `", unseen"` when
`readState === "unseen"`. The Rust projection does not carry read state into
the toast at all — `project_notification` reads only the draft.

This is a real functional difference, not a wording one: a GPUI surface cannot
tell an operator which notifications are new.

**Options.**
1. **Project read state in Rust too.** Requires deciding how a toast shows it
   — `ToastStackSpec` has no unread affordance, so this may need Poodle.
2. Decide read state belongs to the notification centre and not the toast, and
   remove it from the Svelte label.
3. Leave the difference.

**Recommendation: 2.** A toast is transient and appears *because* it is new;
labelling it unseen is close to tautology. But this is a product call.

### D8. The browser is a platform, and Rust has to be handed what it gives away

**What.** Three things the Svelte tier takes for free:

| Fact | Svelte | Rust |
| --- | --- | --- |
| request ids | `crypto.randomUUID()` | injected id source |
| dates | `toLocaleString` | injected `TimestampFormat` |
| locale casing | `toLocaleLowerCase` | see D4 |

`longhorn-poodle::licence` already takes a `TimestampFormat` for this reason.

**Why it is listed.** Not as a defect — the seam is right and matches
`longhorn-gpui-windowing`'s backend seam. It is listed because it means a GPUI
application has three things it must supply that a Tauri application never
had to think about, and nothing currently tells an author that.

**Options.**
1. **A single `HostServices` bundle** carrying id generation, date formatting
   and locale, supplied once at composition.
2. Keep them as separate per-call injections.
3. Document them as a checklist and leave the shape alone.

**Recommendation: 1.** Three is enough to be worth bundling, and a fourth is
likely.

### D9. Toast actions: one shown, several held

**What.** A `NotificationRecord` may carry several actions; a Poodle `Toast`
has room for one. Rust projects the first and documents that the rest are
reachable from the notification centre. The Svelte tier does not project
actions onto toasts at all.

**Options.**
1. **Match Rust** — first action on the toast, rest in the centre.
2. Match Svelte — no actions on toasts anywhere.
3. Decide per action, using a field on the record.

**Recommendation: 1.** A backup-failed toast with a "Retry" button is worth
more than a tidy rule, and "first action" is a rule the domain can state.

## What this memo does not cover

Poodle's own cross-backend parity is Poodle's work and already has apparatus:
`packages/gpui/cross-runtime-parity-report.json`,
`packages/gpui/parity-priority-matrix.json`, and a declared blocker about the
missing mounted GPUI preview app. Nothing here should be duplicated into
Longhorn. This memo is only about **the facts Longhorn projects** and whether
the two backends show them the same way.

This register was derived from reading both implementations rather than from
comparing two rendered surfaces. **Card 171 closed that gap for behaviour**:
`fixtures/parity/projection-v1.json` states thirty-nine cases as data, and
both tiers check against it. Everything the "genuinely settled" section above
claimed by reading now holds by test, and both sides passed first try.

What remains unproven is *rendering*. The two tiers now provably decide the
same things; nobody has looked at a Svelte surface and a GPUI surface showing
the same facts side by side. That is a smaller claim than it was, and still an
open one.

## Decisions required

| | Divergence | Decision | State |
| --- | --- | --- | --- |
| D1 | serde wire form as UI text | fix Svelte | **done** — card 170 |
| D2 | duplicated label maps | generate from Rust | **done** — card 170 |
| D3 | settings rules in the client tier | port now, generate later | **done** |
| D4 | search normalisation | host decides, via `HostServices` | **done** |
| D5 | Critical vs Error collapse | carry in text, both tiers | **done** |
| D6 | re-declared spec shapes | import or generate | open — Poodle-side |
| D7 | read state on toasts | drop from Svelte | **done** |
| D8 | platform services | bundle as `HostServices` | **done** |
| D9 | toast actions | first action, both tiers | **done** — already agreed |

## What was decided and built — 2026-08-09

**D8 — `longhorn_core::HostServices`.** One trait carrying
`new_request_id`, `format_timestamp` and `fold_case`, supplied once at
composition. In `longhorn-core` rather than the projection crate because
request ids belong to the command tier and dates to the projection tier, and
both sit above core. `PlainHostServices` exists for tests and is named to
discourage shipping it — an application that reaches for it is telling its
users that dates look like integers.

**D3 — settings search and deep-link resolution in Rust.**
`longhorn-poodle::settings` gained `search`, `resolve_deep_link`,
`SearchMatch`, `SearchResult`, `ResolvedDeepLink` and `DeepLinkError`. Page
and anchor hits stay distinct destinations. An unknown anchor is an error
rather than a silent fall back to the top of the page: a deep link that
half-works sends someone to the wrong place believing it is right.

**D4 — folded through `HostServices::fold_case`.** Neither tier's standard
library decides any more; the host does. A test proves it by supplying a host
that folds nothing, which makes search case-sensitive — a real answer some
host could want, and proof the projection is not folding behind its back.

**D5 — `CRITICAL_PREFIX`.** A `Critical` record's toast title is prefixed
`"Critical: "`. A word rather than a symbol or a colour, because an icon or a
tint lands in the same vocabulary that already cannot tell the two apart, and
a screen reader announces the word. Marked on the title rather than the
message because a toast is read at a glance and the message may truncate.

If a second domain needs a fifth severity level, the Poodle-tone option
becomes right on cost as well as on shape. This is the cheap correct answer
for one caller, not the permanent one.

**D1 and D2 — Card 170, complete.** Sizing found that not every label can be a
table entry: six of the thirteen restore compatibility arms interpolate their
own fields. The answer was to carry *templates* rather than finished strings,
with both sides filling `{name}` placeholders from the same table. The wording
moved onto the domain enums that own it, which also dissolved the worry about
`longhorn-bindings` depending on `poodle-specs` — a label was never a property
of the Poodle projection.

Three generated artifacts now cross, checked by `check:bindings`, and no
Svelte surface renders a serde `rename_all` output any more. One exception is
written down: `identityLabel` interpolates two fields into a sentence and is
still stated twice.

## D7 and D9 — decided 2026-08-09

**D7 — read state left the toast.** `notificationStatusLabel` no longer
appends `", unseen"`. A toast appears *because* something just happened, so
marking it unseen is close to tautology; unread-ness belongs to the
notification centre, where it can be acted on. No Poodle change, and the
alternative would have needed one — `ToastStackSpec` has no unread affordance.

**D9 needed no code, and this memo had it wrong.** The claim above that "the
Svelte tier does not project actions onto toasts at all" is false.
`NotificationController.projectToast` takes `record.draft.actions[0]` — the
same first-action rule Rust uses. The two tiers already agreed; only this memo
did not.

The rule is now stated once as `toastAction` in `packages/longhorn`, and the
parity fixture checks it on both sides with three cases: no actions, one, and
three.

### A third tone implementation, found while checking

`NotificationController.projectToast` carried its own copy of the
severity-to-tone rule — `severity === "error" || "critical" ? "danger" :
severity` — inline, in the client tier, while the Poodle projector had another
and Rust a third. The parity fixture checked the projector, so the client
tier's copy was unchecked and could have drifted without a red gate.

Moved to `notificationSeverityTone` in `packages/longhorn`, which both
TypeScript callers now use and the parity fixture checks. Found only because
D9 sent me to read `projectToast`.

## D5, finished properly — 2026-08-09

The first pass applied the `Critical: ` prefix in Rust only, and the parity
fixture recorded the gap as a deliberate difference. It was not deliberate; it
was half a decision. "Longhorn carries the distinction in text" is about
Longhorn, and the Svelte tier is Longhorn.

The prefix moved onto `NotificationSeverity::title_prefix`, where it belongs —
what a severity needs said about it is a property of the severity, the same
argument Card 170 made for labels. It generates into
`NOTIFICATION_SEVERITY_TITLE_PREFIXES`, `NotificationController.projectToast`
applies it through `toastTitle`, and the parity fixture checks three cases on
both sides.

The map is partial by design: a severity absent from it needs no prefix
because its tone says enough. Only `Critical` is in it.

**`deliberateDifferences` is now empty.** The test that reads it no longer
requires it to be non-empty — a suite that lists only agreements reads as
though there are no differences, but so does one that invents them.

## What is still open

D6 alone. It is Poodle-side work in either form, and blocks nothing.
