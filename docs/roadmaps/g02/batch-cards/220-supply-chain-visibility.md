# 220 Supply-chain Visibility

Status: ready
Owner: Tom
Roadmap: g02.026 batch 3
Governing refs: contract 012; memo 023 (M-advisories, L8, L9, L11 hygiene
lane)
Depends on: none
Auto-start next card: no

## Objective

A future vulnerability-class advisory is visible the day it publishes, the
committed generated schemas have a policy, and CI runs on pinned tools.

## Why this exists

A live `cargo deny check advisories` fails on 13 advisories — all
`unmaintained`-class, all transitive via Tauri (8× GTK3 bindings via wry
0.55.1, 5× `unic-*` via urlpattern, proc-macro-error). Zero
vulnerability-class today, and no gate — so a future real advisory is
invisible until someone runs the tool by hand. Also: 20 committed generated
Tauri schema files (~1.15 MB) with no `.gitignore` and no freshness gate
(Tauri's own template gitignores them); CI floats on `stable` with no
`rust-toolchain.toml`; workflow actions are unpinned (`release.yml:60-62`
self-acknowledged TODO); `npm install -g npm@latest` runs in a publish-rights
step (`release.yml:108`).

## Scope

- `deny.toml` (new) + CI wiring
- `examples/*/src-tauri/gen/schemas/` policy
- `.github/workflows/` pinning (with approval)

## Steps

1. Add `deny.toml`: the 13 known unmaintained advisories explicitly allowed
   with dates and reasons (all upstream-Tauri, none actionable today);
   vulnerability-class stays deny. Wire `cargo deny check advisories` into CI
   or release gates.
2. `gen/schemas`: decide ignore-and-regenerate or commit-with-freshness-gate;
   implement. Tauri's own template ignores them — deviation needs a reason.
3. Pin the CI toolchain (floor job already pins 1.95.0; the rest float) and
   pin workflow actions to SHAs. Replace `npm install -g npm@latest` with a
   pinned version in the publish step. (Workflow edits — explicit approval.)
4. Redundant workspace `exclude` entries (`Cargo.toml:54-57` — all seven
   prototypes carry their own `[workspace]`): remove or comment why.

## Do Not

- Blanket-allow `unmaintained` forever. Each allow is named, dated, and has a
  revisit trigger (Tauri upgrade).
- Treat the advisory gate as blocking publication on upstream's unmaintained
  GTK3 stack — that is a Tauri dependency decision, not this card's.

## Acceptance Criteria

- [ ] `cargo deny check advisories` runs in automation and fails on any new
  advisory outside the dated allow list
- [ ] the schemas policy is implemented and gated
- [ ] CI runs on pinned toolchains and actions

## Evidence Required

- the `deny.toml` with its allow rationale
- the gate run
- the workflow diff (approved)

## Stop Conditions

Stop on `.github/workflows/` edits without explicit approval.
