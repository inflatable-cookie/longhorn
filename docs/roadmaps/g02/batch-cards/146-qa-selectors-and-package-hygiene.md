# 146 QA Selectors And Package Hygiene

Status: complete
Owner: Tom
Roadmap: g02.006 batch 1
Governing refs: contracts 001 and 012; research memo 018
Depends on: none
Auto-start next card: no
Completed: 2026-08-03

## Objective

Make every QA selector resolve, complete the check aggregates, and settle
package and manifest conventions.

## Scope

- `effigy.toml` history-persistence selectors and check aggregates
- `packages/svelte`, `packages/bridge`, `packages/layout` manifests
- workspace `Cargo.toml` version-field convention and pins

## Steps

1. Point `qa:northstar:g01-history-persistence` and
   `qa:northstar:g01-history-tree-persistence` at the real fixture files (or
   add the intended `linear-v1.json`/`tree-v1.json` fixtures if the checks
   meant to cover them).
2. Add `check:history-tree-bindings` to the bindings aggregate and a
   `check:layout-ts` task to `check:client-ts`.
3. Align `packages/svelte`'s svelte peer range to the workspace `<6`
   convention; demote `packages/bridge`'s `@longhorn/tauri` to the
   operation-style optional-peer subpath pattern; adopt `workspace:*` for
   internal deps.
4. Pick one internal path-dep convention in `Cargo.toml` (version on all or
   none) and apply it; record a deliberate decision on the `rusqlite =0.31.0`
   pin.

## Acceptance Criteria

- both history persistence selectors pass
- aggregates cover history-tree bindings and layout TS
- package installs and consumer `file:` resolution unchanged; nucleus
  boundary verifier unaffected
- full `effigy qa` passes

## Evidence Required

- selector pass receipts
- convention decision records
- QA receipts

## Stop Conditions

- `workspace:*` or peer changes break a consumer's `file:` install
- the rusqlite refresh demands API migration beyond a pin bump

## Evidence

- finding B7 retracted with passing-selector evidence; memo 018 corrected
- bindings and client-ts aggregates completed (+ new `check:layout-ts`,
  which surfaced and fixed three latent test type errors)
- svelte peer-range finding retracted (deliberate proven-upper-bound
  policy, test- and guide-pinned); bridge optional-peer demotion attempted,
  reverted, and deferred — it invalidates the frozen Card 127 candidate
  receipt (pinned tarball digest and empty peer matrix); recorded for the
  next distribution candidate;
  `workspace:*` rejected (consumer `file:` installs) and recorded; all
  internal Cargo path deps versioned; rusqlite pin decision recorded
- log: `docs/logs/2026-08/03-qa-selectors-package-hygiene-and-front-doors.md`

## Next Task

Promote Card 147.
