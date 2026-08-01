# 106 Loophole Display And Window Host Cutover

Status: planned
Owner: Tom
Roadmap: g01.015 batch 2
Governing refs: contracts 002, 003, 009, 010, and 012; Card 105
Depends on: Card 105
Auto-start next card: no

## Objective

Replace generic Echo/Aura display correlation, window diff, apply, capture,
settling, persistence, reveal, and shutdown mechanics with the Longhorn host.

## Repository Scope

- Longhorn: only admitted host fixes and donor fixtures.
- Loophole: Aura/Echo workspace-window adapters, capabilities, tests, and docs.
- Plugin/native host repos and code: unchanged.

## Scope

- canonical local display inventory and arrangement signature
- protected hidden `main` host plus dynamic `workspace-*` windows
- geometry per display arrangement, title, maximize, close, and display adoption
- exact apply generations, readback, attribution, debounce, flush, and receipts
- distinct registered window-placement domain
- retained plugin/native window exclusion

## Steps

1. Freeze donor geometry, retag, fallback, display-loss, and close traces.
2. Bind current Tauri observation to canonical display facts.
3. Compose protected and dynamic window roles through one host assembly.
4. Replace timer-only origin heuristics with Longhorn generations and settling.
5. Gate reveal on native convergence and renderer readiness.
6. Preserve Loophole display adoption and last-Surface window policy as adapters.
7. Remove superseded workspace-window apply mechanics and capabilities.
8. Prove multi-display restart, loss/return, close, and aggregate shutdown.

## Acceptance Criteria

- `main` is never recreated or accidentally closed
- dynamic windows converge or return exact partial-failure evidence
- user geometry survives restart without apply-loop echo
- window placement cannot overwrite Surface or layout state
- plugin editor and other non-workspace windows remain outside the host
- no three/five-second heuristic remains as native authority

## Stop Conditions

- canonical display identity depends on renderer/Tauri ids
- Loophole close policy must move into Longhorn
- failed flush or reveal would be reported as success

## Evidence Required

- donor/target window traces and exact source receipts
- multi-display restart and loss/return report
- generation, capture, flush, reveal, close, and shutdown receipts
- capability and duplicate-mechanism audit

## Next Task

Execute Card 107's registered eight-region layout cutover.
