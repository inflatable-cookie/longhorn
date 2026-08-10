# 149 Distribution Candidate V2

Status: blocked — stop condition fired; needs a coordinated re-freeze with Poodle
Owner: Tom
Roadmap: g02.008 batch 1
Governing refs: contracts 001, 003, 012, and 013; Card 127 receipt
Depends on: Card 148
Auto-start next card: no

## Objective

Freeze the second private compatibility candidate over the refreshed graph
and clear the two deferrals parked behind the Card 127 receipt.

## Scope

- bridge `@inflatable-cookie/longhorn-tauri` demotion to optional peer (package test,
  topology artifact proof, proof consumers)
- new candidate fixture, verifier, and receipt superseding Card 127
- refreshed 18-package/38-crate counts and dependency graph
- diagnostics-seam adoption section in the composition guides

## Steps

1. Demote the bridge peer and update the three pinned evidence layers.
2. Regenerate the candidate fixture and verifier over the current tree;
   archive the Card 127 receipt with a supersession pointer.
3. Re-run the packaged candidate proofs.
4. Add diagnostics-seam adoption guidance with one consumer-shape example.
5. Full `effigy qa` plus the candidate verifier.

## Acceptance Criteria

- optional-peer shape asserted end to end
- candidate verifier passes against the live tree
- superseded receipt archived, not rewritten
- full `effigy qa` passes

## Evidence Required

- new receipt digest and verifier receipts
- supersession record
- QA receipts

## Stop Conditions

- a consumer `file:` install breaks on the peer shape
- Poodle artifact set drift forces a coordinated re-freeze

## Blocker

**The stop condition fired.** This card names "Poodle artifact set drift forces
a coordinated re-freeze", and that is where it now stands.

The earlier blocker — consumer trees dirty — cleared. Every consumer manifest
is clean. Running the verifier then found the receipt describing a world that
had moved on in four independent ways, three of which are now fixed.

### What had gone stale

**TypeScript package count.** The verifier pinned 18; g02.013 consolidated the
tier to 3, so it refused to run against the repository it describes. Fixed: the
counts are derived, because the receipt already enumerates every package and is
compared whole. A literal froze the graph twice and failed on the wrong one —
the same defect `verify-guides-card126.ts` had, recorded in `PAPERCUTS.md`.

**The private consumer's path was a placeholder in executable code.** Commit
`6a84574c docs: remove third-party identity so the repo can be made public`
replaced a real path with `../<private-consumer>` in `consumers.ts`, not in
prose. The verifier has been unable to resolve it ever since, so this card has
been recorded as held on consumer threads while the real reason was that the
gate could not run at all. Fixed: the path comes from
`LONGHORN_PRIVATE_CONSUMER`, and an unset value records a **named omission** in
the receipt rather than silently covering one graph fewer.

**Loophole was restarted greenfield.** The application this candidate has
always described is now `loophole-legacy` — stabilised, and still a real
consumer. The `loophole` directory holds a days-old greenfield app that is mid
architecture. Fixed: the entry points at `loophole-legacy`. The greenfield app
is a known consumer deliberately not yet pinned, because freezing a
compatibility claim about something being actively designed produces a claim
that is wrong tomorrow.

**Poodle's artifact set changed, and is changing now.** `packages/styles` and
`packages/svelte/icons-lucide` no longer exist; the vendored Lucide catalogue
is being removed as this is written. This is the one that is not fixed, and not
fixable from here.

### Resume

A coordinated re-freeze, in this order:

1. Poodle settles its package set and says what it is.
2. Update the artifact family list in
   `scripts/private-candidate-card149/artifacts.ts` to match.
3. `LONGHORN_PRIVATE_CONSUMER=<path> bun scripts/verify-private-candidate-card149.ts --write`
4. Wire the card149 generate/proof/docs-check tasks into `effigy.toml`.
5. Supersede the Card 127 receipt with a pointer — archived, not rewritten.
6. Refresh the candidate reference doc and CHANGELOG, then full QA.

### What this says about the shape

A receipt that pins five external repositories goes stale at the rate of the
fastest-moving one, and nothing tells you until you run it. Four sources of
drift accumulated silently between one attempt and the next. Whatever replaces
this should either run often enough to fail early, or pin fewer things.

## Progress

Part 1 is complete and committed: the bridge `@inflatable-cookie/longhorn-tauri` dependency
is an optional peer, asserted by the bridge package test, the
bridge-topology and operation-notification artifact proofs, and the five
proof consumers; the card149 candidate verifier exists with truthful
18-package/38-crate counts; the diagnostics-seam adoption section is in
`docs/guides/system-composition.md`.

## Gate

Receipt generation freezes consumer graphs and asserts clean *selected*
manifests — not whole trees. Poodle passes with 219 dirty files because
its five packaged directories are clean; the check is precise.

Nucleus, loophole, jetstream, figmatic, and kimi-shell are now clean. The
remaining blocker is soundcheck: an uncommitted `zip = "8.6.0"` line in
its root `Cargo.toml`, owned by the soundcheck thread. Resume by running `bun scripts/verify-private-candidate-card149.ts
--write` once nucleus settles, then wire the card149
generate/proof/docs-check tasks into `effigy.toml`, supersede the Card 127
receipt with a pointer, refresh the candidate reference doc and CHANGELOG,
and run full QA.

## Next Task

Poodle's package set. Everything else is ready and the verifier reaches the
artifact stage on the first run now.
