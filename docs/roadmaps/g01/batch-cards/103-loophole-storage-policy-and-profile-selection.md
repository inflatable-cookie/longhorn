# 103 Loophole Storage Policy And Profile Selection

Status: operator-held
Owner: Tom
Roadmap: g01.015 batch 1
Governing refs: contracts 001, 003, 004, and 012; Card 102;
`../../../architecture/loophole-migration-map.md`
Depends on: Card 102
Auto-start next card: no

## Objective

Select, contract, implement, and prove one profile for Loophole's shared
multi-process product root without per-purpose path overrides.

## Repository Scope

- Longhorn: contract, resolver, fixtures, tests, package evidence, and docs.
- Loophole: Chorus storage contract may change only after the operator choice;
  application code remains read-only.
- External package registries: out of scope.

## Operator Decisions

1. Windows root: keep Chorus `%APPDATA%\Loophole` roaming parent, or revise the
   product contract to Longhorn's local-data parent.
2. Linux leaf: keep lowercase `loophole`, or use the exact stable storage name
   `Loophole` on every platform.

Recommended: keep the shared durable product parent on each platform and use
one exact `Loophole` leaf everywhere. That produces a reusable
`shared-product-root-v1` profile and avoids platform-specific identity.

## Scope

- canonical id `com.inflatablecookie.loophole`
- explicit stable storage name `Loophole`
- exact macOS, Windows, and Linux root matrix
- typed `config`, `data`, `state`, `cache`, `logs`, `runtime`, and `backups`
- fixed canonical-id bootstrap locator
- profile diagnostics and compatibility identity

## Steps

1. Record both operator choices in Longhorn and Chorus contracts.
2. Amend contract 004 if the selected matrix needs a new profile or directory fact.
3. Implement deterministic resolution without per-purpose overrides.
4. Add full platform, identity, bootstrap, and transition fixtures.
5. Prove existing profile ids do not change.
6. Pack the affected private artifacts and record exact receipts.

## Acceptance Criteria

- one versioned profile produces the selected matrix exactly
- the canonical id remains app and locator identity
- stable `Loophole` storage identity is explicit and receipted
- no display-name derivation or silent case normalization exists
- cache/log placement warnings are explicit when the profile unifies roots
- existing profile results remain byte-for-byte stable
- no Loophole application code changes

## Evidence Required

- operator decision and contract diff
- cross-platform resolver fixture
- compatibility and transition tests
- private artifact receipt
- focused Northstar validation

## Stop Conditions

- the selected matrix remains ambiguous
- implementation needs hidden per-purpose overrides
- an existing profile id would change meaning
- process sharing requires undeclared multi-writer authority

## Next Task

Execute Card 104. Restore clean Loophole baseline health and admit the exact
private dependency graph.
