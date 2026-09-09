# g02.008 Distribution Candidate V2

Status: blocked — awaiting a coordinated Poodle re-freeze
Owner: Tom
Updated: 2026-08-04
Governing refs: contracts 001, 003, 012, and 013; Card 127 receipt; g02
candidate runway; absorbed Card 149 (blocked card record below)
Depends on: g02.007

## Outcome

Freeze a second private compatibility candidate over the refreshed graph,
superseding the Card 127 receipt: bridge `@inflatable-cookie/longhorn-tauri` demoted to an
optional peer, truthful 18-package/38-crate counts, and re-frozen
commit-pinned proofs. Includes the diagnostics-seam adoption guidance.

## Generation Runway

Eighth g02 task, closes the Tier A lane.

## Work

Absorbed from Card 149 (blocked; stop condition fired — needs a coordinated
re-freeze with Poodle). Single remaining outcome: freeze the second private
compatibility candidate over the refreshed graph and clear the two deferrals
parked behind the Card 127 receipt.

### Done: part 1

The bridge `@inflatable-cookie/longhorn-tauri` dependency is an optional peer, asserted by
the bridge package test, the bridge-topology and operation-notification
artifact proofs, and the five proof consumers; the card149 candidate verifier
exists with truthful package/crate counts; the diagnostics-seam adoption
section is in `docs/guides/system-composition.md`.

Three of the four staleness kinds the verifier found are fixed: counts are
derived (the receipt enumerates every package and is compared whole); the
private consumer path comes from `LONGHORN_PRIVATE_CONSUMER`, with an unset
value recording a named omission rather than silently covering one graph
fewer; the entry points at `loophole-legacy`, with the greenfield app
deliberately unpinned while it is being designed.

### Blocked: receipt generation

Poodle's artifact set changed and is changing (`packages/styles` and
`packages/svelte/icons-lucide` gone; the vendored Lucide catalogue being
removed). Not fixable from here. Resume, in order:

1. Poodle settles its package set and says what it is.
2. Update the artifact family list in `scripts/private-candidate-card149/artifacts.ts` to match.
3. `LONGHORN_PRIVATE_CONSUMER=<path> bun scripts/verify-private-candidate-card149.ts --write`
4. Wire the card149 generate/proof/docs-check tasks into `effigy.toml`.
5. Supersede the Card 127 receipt with a pointer — archived, not rewritten.
6. Refresh the candidate reference doc and CHANGELOG, then full QA.

A receipt that pins five external repositories goes stale at the rate of the
fastest-moving one. Whatever replaces this should either run often enough to
fail early, or pin fewer things.

### Acceptance criteria

- optional-peer shape asserted end to end (done: bridge package test, topology artifact proof, proof consumers)
- candidate verifier passes against the live tree
- superseded receipt archived, not rewritten
- full `effigy qa` passes

### Evidence required

- new receipt digest and verifier receipts
- supersession record
- QA receipts

### Stop conditions

- a consumer `file:` install breaks on the peer shape
- Poodle artifact set drift forces a coordinated re-freeze (fired)

### Gate

Receipt generation freezes consumer graphs and asserts clean *selected*
manifests — not whole trees. Nucleus, loophole, jetstream, figmatic, and
kimi-shell are clean; soundcheck carries an uncommitted `zip = "8.6.0"` line
owned by the soundcheck thread.

## Absorbed records

- Card 149: blocked — stop condition fired; coordinated Poodle re-freeze outstanding. Scope, steps, acceptance, evidence, and stop conditions absorbed inline above.

## Goals

- [x] bridge main entry ships without a hard `@inflatable-cookie/longhorn-tauri` dependency
- [ ] candidate receipt reflects the current package/crate sets and the
  refreshed dependency graph
- [ ] Card 127 receipt superseded, not silently rewritten
- [x] consumers get one diagnostics-seam adoption reference

## Acceptance Criteria

- [ ] bridge package test, topology artifact proof, and proof consumers
  assert the optional-peer shape
- [ ] new candidate fixture and verifier pass; the superseded receipt is
  archived with a pointer
- [ ] full `effigy qa` passes

## Explicit Non-goals

- registry publication (still deferred)
- Poodle artifact set changes
- consumer repository edits

## Next Task

A coordinated re-freeze with Poodle, once its package set settles. The resume
sequence lives in Work above.

The hold recorded here as "operator-held on nucleus quiescence" was wrong for
most of its life: consumer trees cleared, and the verifier then failed on four
kinds of staleness it could not previously reach — including a redaction sweep
that replaced a consumer path with a placeholder in executable code. Three are
fixed. The fourth is Poodle's artifact set, which is the stop condition this
card always named.
