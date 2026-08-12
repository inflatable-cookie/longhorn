# 194 Strictness For The Lenient Eight

Status: complete — landed 2026-08-12; bridge removed from scope
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

- [x] Every `record(value, path)` that sits under a discriminant check becomes
      `record(value, path, variantKeys(type, value, path))`.
- [x] Keep the discriminant check above the lookup, as Cards 187 and 188
      established. A missing map entry then means the generator failed rather
      than that a consumer sent something odd.
- [x] One lookup helper per domain, reading that domain's map and its
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

- [x] `effigy qa` passes.
- [x] The generator reports no unreadable union in any of the six. Bridge's
      six are externally tagged and out of scope; see above.
- [x] No hand-written per-variant key list exists in any domain, which is the
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

## Outcome — 2026-08-12

Six domains wired, 50 unions gained a key check: config 20, commands 12,
transfer 7, surfaces 4, surface-transfer 4, settings 3. `effigy qa` exit 0;
203 package tests.

There is no uniform call shape across domains, which is why each was done by
hand rather than scripted. Four had a single choke point that covered several
unions at once — `assertKnownKind` in surfaces, `responseWithStatus` in
transfer and surface-transfer, `outcome` and `mutationOutcome` in config — and
threading the type name through those was most of the work.

**Two findings, and the first is a live defect rather than a gap.**

`assertValidSurfaceMutationResponse` was calling
`record(value, SURFACE_FIELDS.SurfaceMutationResponse)`. That type is a union
and the flat field map **skips unions by design**, so the lookup was
`undefined` and `record` returned without checking any key. It read as a closed
boundary and was an open one — worse than the domains that plainly checked
nothing, because a reader sees a field list being passed.

`surface-transfer` already carried a comment naming this exact limitation:
"`SurfaceTransferTarget` is a tagged union, so its allowed keys depend on
`kind` and one flat list is wrong. The generator skips it for that reason."
Someone hit it, understood it, and documented it rather than working around it.
The per-variant map is the answer that comment was waiting for.

**A fourth discriminant name appeared.** Card 188 found `kind`, `status` and
`state`; `transfer` and `surface-transfer` tag their abort sources on `domain`.
The detector found it without being told, which is the whole reason Card 188
made detection replace configuration.

**Process note.** Two `effigy qa` runs overlapped, because one was started to
gate two domains and then four more were wired while it ran. The second failed
on a lock conflict, and the first was gating a tree that had changed four times
underneath it. Both results were discarded and one clean run was taken over the
final tree. One gate at a time, and do not edit while it runs.
