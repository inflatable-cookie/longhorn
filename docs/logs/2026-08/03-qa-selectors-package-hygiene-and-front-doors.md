# QA Selectors, Package Hygiene, And Front Doors

Date: 2026-08-03
Cards: 146-147
Roadmap: g02.006

## Result

Check aggregates are complete, package conventions are settled and recorded,
one audit finding is retracted with evidence, and every prose front door
matches repo state.

## Shape

- Audit finding B7 retracted: the history persistence selectors reference
  crate-relative fixtures that exist; both selectors pass unchanged. Memo
  018 corrected in place (as is the svelte peer-range finding below).
- `check:bindings-card127` now includes `check:history-tree-bindings`;
  `check:client-ts` gained `check:layout-ts` (new task) plus
  `check:history-tree-ts`. The new layout typecheck immediately surfaced
  three latent test-only type errors, now fixed.
- Second retraction: `packages/svelte`'s `<=5.56.8` svelte peer cap is
  deliberate proven-upper-bound policy (package test and compatibility
  guide pin it); left unchanged.
- Bridge optional-peer demotion attempted and reverted: three layers of
  frozen evidence pin the current shape — the bridge package test, two
  artifact proofs, and the Card 127 candidate receipt (exact tarball digest
  and empty peer matrix). The observation stands (the main entry is
  transport-agnostic); executing it belongs to the next distribution
  candidate card, not remediation. Deferred with rationale in memo 018.
- Decision: internal TS deps stay pinned `"0.1.0"` — `workspace:*` would
  break consumer `file:` installs that resolve these packages outside the
  workspace. Recorded here instead of adopted.
- All internal Cargo path deps now carry `version = "0.1.0"` (one
  convention; publish-ready).
- Decision: `rusqlite =0.31.0` pin retained for now; upgrading crosses API
  and bundled-SQLite changes that need their own bounded card. Deferred
  candidate, not silent drift.
- Front doors: README current state rewritten (g01 complete, g02 active,
  compact); CHANGELOG counts corrected to 18 packages / 38 crates with
  fork-tree, diagnostics-seam, and g02 remediation entries; contract-index
  readiness and `Updated:` header refreshed; contract 004 header bumped;
  g01 batch-card index moves Card 074 to Complete and defers its next-task
  pointer to the generation index; `docs/reference/api-surface.md`
  regenerated and its check passes.

## Exact Evidence

- `qa:northstar:g01-history-persistence` and
  `qa:northstar:g01-history-tree-persistence` pass
- `check:bindings-card127`, `check:client-ts`, and
  `check:api-reference-card126` pass
- full `effigy qa` green over the complete g02 batch
