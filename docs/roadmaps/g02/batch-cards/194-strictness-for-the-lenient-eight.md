# 194 Strictness For The Lenient Eight

Status: in progress — step 1 complete 2026-08-12; bridge removed from scope
Owner: Tom
Roadmap: g02.018 batch 3
Governing refs: contract 010; contract 011; contract 012
Depends on: Card 187 (complete); Card 188 (complete)
Blocks: batch 4
Auto-start next card: no

## Why

Eight domains check a tagged union's discriminant and then each variant's
fields, and never the variant's **key set**. An unknown key on a variant is
accepted today.

`config`'s `assertValidStorageTransitionInspectOutcome` is the shape: it calls
`outcome(value, STORAGE_TRANSITION_INSPECT_STATUSES, "$")`, then
`record(value, "$")` with no field list, then checks fields individually. The
`record` call is where the key set would go and does not.

## The Decision This Batch Waited On

Answered 2026-08-12. It was the first batch that could reject a payload a
shipped consumer sends today, so it was held for the operator.

Measured rather than argued: Loophole makes zero direct `longhorn_*`
invocations, Longhorn's own controllers build the payloads where a controller
exists, and the `kind:` literals in consumer code are their own app domains.
A payload this batch would newly reject is one TypeScript already disallows.
Four consumer repos of seven were checked, and that residual is the known risk.

**What this batch must not inherit:** Card 188 verified 44 of its 69 replaced
lists only at the attribution-free standard. Nobody has checked those per
variant, so "the earlier domains were audited" is not available as a reason to
skip care here.

## Scope

**Six domains**, not seven. `commands`, `config`, `settings`, `surfaces`,
`surface-transfer`, `transfer`. `history` already had its variant map, added
when Card 191 needed one for the page floor.

### Bridge is out, on this card's own stop condition

Its unions are **externally tagged**, not internally tagged:

```ts
export type BridgeDeduplicationSupport = "unsupported" | { "finite": BridgeDeduplicationCapacity };
```

There is no shared discriminant key — the variant *is* the key. Card 188's
detector reports six such unions and produces nothing for them, which is the
correct answer rather than a failure: there is no discriminant to detect.

`BridgeQueryOutcome<S, D>`, `BridgeCommandOutcome<S, D>`,
`BridgeDeduplicationSupport`, `BridgeJobTerminalOutcome<S, D>`,
`BridgeCancellationOutcome<D>`, `BridgeServiceOutcome`.

Bridge needs either internal tagging or a validation strategy shaped for
external tagging, and which is a modelling decision. Batch 4's build check must
exempt it explicitly rather than by silence.

## Step 1 — Emit the map everywhere

- [x] Each generator renders a variant map beside its flat one,
      with `variant_field_map`. No new generator code; Card 187 built it to be
      called from twelve places and Card 188 made the discriminant detected
      rather than supplied.
- [x] This step is inert. A map nobody reads changes no behaviour, so it can
      land on its own commit and be verified by the generated output alone.
- [x] Any union the detector cannot read is reported by name and left for
      step 2 to handle explicitly, not silently skipped. Six, all in bridge,
      all externally tagged — see the scope note.

Landed 2026-08-12. 168 unions across twelve domains now have a generated
per-variant map: config 40, commands 24, native-content 20, operation 16,
transfer 14, history-tree 10, notifications and surfaces and surface-transfer
and update 8 each, history and settings 6 each.

Two wiring faults, both caught by the compiler rather than by review. Four
generators carry a `Vec<String>` in their `RenderedProtocol` so the struct edit
skipped them, and `config` builds its artifacts inline from `all_declarations`
with no struct at all, so its map was declared and never written.

## Step 2 — Read it where the discriminant is already checked

- [ ] Every `record(value, path)` that sits under a discriminant check becomes
      `record(value, path, variantKeys(type, value, path))`.
- [ ] Keep the discriminant check above the lookup, as Cards 187 and 188
      established. A missing map entry then means the generator failed rather
      than that a consumer sent something odd.
- [ ] One lookup helper per domain, reading that domain's map and its
      discriminant names. Do not hand-write a key list anywhere.

## Step 3 — Say what each domain gained

This batch adds checks rather than exchanging them, so the interesting output
is what is now rejected that was not.

- [ ] Per domain, a test sending one variant with exactly its declared keys and
      the same variant with one surplus key.
- [ ] Per domain, the batch log names how many unions gained a key check.
- [ ] If any domain's existing tests fail, that is a payload a fixture was
      sending which the enum does not declare. Report it rather than widening
      the map — a fixture is a consumer too.

## Acceptance

- [ ] `effigy qa` passes.
- [ ] The generator reports no unreadable union in any of the seven.
- [ ] No hand-written per-variant key list exists in any domain, which is the
      milestone's first goal and becomes checkable at batch 4.
- [ ] Each domain has the pair of tests from step 3.

## Evidence

- [ ] The tests above, named in the batch log.
- [ ] The count of unions that gained a key check, per domain.
- [ ] Any fixture that had to change, with what it was sending.

## Stop Conditions

- Stop if a domain's existing tests fail in a way that means a **consumer**,
  not a fixture, sends an undeclared key. That is the risk the operator
  decision accepted as residual, and finding it converts an accepted risk into
  a live one worth reporting before continuing.
- Stop if a union's arms do not share one discriminant. Card 188's detector
  handles `kind`, `status` and `state`; a union it cannot read is a modelling
  question rather than a validation one.

## Continuation

Batch 4 turns the rule into a build check: no union missing from a variant map.
It cannot run until this card leaves nothing skipped.
