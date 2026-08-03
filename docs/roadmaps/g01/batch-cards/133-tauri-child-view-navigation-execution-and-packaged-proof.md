# 133 Tauri Child-view Navigation Execution And Packaged Proof

Status: complete
Owner: Tom
Roadmap: g01.020 batch 2
Governing refs: contracts 012 and 017; Card 132
Depends on: Card 132
Auto-start next card: yes
Completed: 2026-08-03

## Objective

Execute admitted navigation through the retained private Tauri webview and
prove real macOS document changes without recreation.

## Scope

- Tauri current-URL observation and navigation
- same-origin allowed and cross-origin denied proof
- same-URL no-reload proof
- retained generation, handle, visibility, bounds, and teardown
- packaged evidence and support ledger

## Out Of Scope

- consumer commands or renderer protocol
- back, forward, reload, or browser-history storage
- non-macOS support claims

## Steps

1. Implement the two narrow runtime operations over the private webview.
2. Extend the controlled proof server with a second document.
3. Prove submitted navigation, readiness transition, and final current URL.
4. Repeat the same URL and prove no second page load.
5. Prove denial causes no request and the child generation remains current.
6. Rebuild and run the packaged macOS proof; refresh exact evidence.

## Acceptance Criteria

- one retained child loads the second document exactly once
- the same request is unchanged and does not reload
- cross-origin denial performs no native navigation
- navigation receipt never claims page-load completion
- existing lifecycle, geometry, capability, and scale evidence stays exact

## Evidence Required

- packaged report and transcript
- updated inventory and verifier
- focused proof build/run/check

## Stop Conditions

- runtime URL read is unavailable on the retained child
- navigation bypasses the installed policy callback
- proof needs an external network source

## Next Task

Card 134 proves produced artifacts and records consumer resume gates.

## Evidence

- the Tauri runtime reads and navigates through its retained private handle
- the packaged macOS proof changes from `/proof` to `/navigated` on generation
  1 without attach or close
- repeating the exact URL returns `unchanged` and the server observes one
  `/navigated` request
- a cross-origin URL is denied before native navigation
- page-load start and finish each occur once and preserve the current child
- the refreshed run has eight passing checks, one honest observation unknown,
  and one environment-unmet live-scale claim
- `effigy qa:native-content-child-view-proof` passes
