# 195 Make The Rule A Check

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.018 batch 4
Governing refs: contract 010; contract 011; contract 012
Depends on: Card 194 (complete)
Auto-start next card: no

## Why

Three cards established that a tagged union's allowed keys come from the enum.
Nothing enforces it. The next person to add a union, or to reach for a literal
key list under a deadline, gets no resistance — and the failure is silent, as
`checkoutBranchRoot` and `SurfaceMutationResponse` both were.

## Step 1 — An unreadable union fails the build

`variant_field_map` already returns the unions it could not read, and every
generator prints them. A print is not a check.

- [x] A union with no detectable discriminant is an error, not a warning.
- [x] **Bridge is exempted explicitly, with its reason in the code.** Its
      unions are externally tagged — `"unsupported" | { "finite": … }` — so
      there is no shared discriminant and no map to require. Exempting it by
      silence would look identical to the bug this check exists to catch.
- [x] The exemption names the six unions rather than the domain, so a *new*
      bridge union still has to be considered rather than inheriting the pass.

## Step 2 — Stop warning about a by-design skip

`field_map` skips every union, because a union has no single key set. Eleven
domains print that as a warning on every generate, and nothing can act on it.

- [x] Remove the warning. The tagged unions are in the variant map now, and
      the string unions are in each domain's `*_KINDS`, `*_CODES` and
      `*_STATUSES` constants. Nothing it reports is actionable.
- [x] `history-tree` already dropped it in Card 187; this is the same change
      for the other eleven.

## Step 3 — No hand-written key list comes back

- [x] A test asserts no literal array is passed as an allowed-keys argument in
      any domain's validation module.
- [x] Scoped to the argument position, not to array literals generally — a
      validator legitimately writes `["preset", "replacement"]` as a
      *discriminant* list where no generated constant exists.
- [x] The failure message says to use the generated map, because the next
      person to hit this will be reaching for the shortcut the last three cards
      removed.

## Acceptance

- [x] `effigy qa` passes.
- [x] Deleting a variant map entry fails the build rather than loosening a
      boundary. Proved by doing it, not by assertion.
- [x] The generator prints nothing on a clean run.
- [x] Bridge's exemption is visible in the source with its reason.

## Evidence

- [x] The tests above, named in the batch log.
- [x] The generator's output on a clean run: empty.

## Stop Conditions

- Stop if the argument-position check cannot distinguish an allowed-keys list
  from a discriminant list. A check that fires on legitimate code gets
  suppressed, and a suppressed check is worse than none.

## Continuation

The milestone closes. Bridge's tagging remains an open modelling question,
recorded on g02.018 rather than here.

## Outcome — 2026-08-12

The rule is a check. `variant_field_map` now asserts rather than returns a
report, so the twelve `if !unreadable.is_empty()` blocks are gone and the rule
lives in one place — which is what Card 187 built it to be called from twelve
places for.

Bridge's six unions are exempted by name in `EXTERNALLY_TAGGED`, compared
without type parameters so `BridgeQueryOutcome<S, D>` matches. Proved by
removing `BridgeServiceOutcome` from the list: the generator panicked and named
it. A test does the same permanently, and a second asserts a *new* bridge union
does not inherit the pass.

Twelve generators dropped the flat map's skip warning. The generator is now
silent on a clean run.

**Step 3 found four hand-written key lists still live**, all in
`history/validation.ts` — the target's `checkout` and non-`checkout` arms, and
the navigation result's two statuses. History was the domain Card 194 excluded
from scope as "already had its variant map", and it had the map without reading
it in three of four places. The milestone's first goal was therefore not met
until this card, and nothing before the check would have said so. All four now
read `variantKeys`; the generated map already held the right keys, including
`checkoutRoot`.

**The stop condition nearly fired.** Scoping by argument *type* was not enough:
`member`, `oneOf`, `assertKnownKind` and `responseWithStatus` all take
`readonly string[]`, and all four legitimately receive literals. The first
version passed only because no such call happened to sit in a matched position.
Scoping is by parameter *name* — `allowed`, `expected`, `keys` — with the
discriminant names excluded in the source and the reason given.

`tests/fixtures/planted-key-list.ts` carries one of each in one file, so the
scan is proved to bite rather than assumed to. A parameter-matching bug reads
exactly like a clean codebase, which is this milestone's own subject.

`effigy qa` exit 0 on one clean run: 214 package tests, 1224 Rust tests.

g02.018 closes. Bridge's external tagging stays open on the milestone.
