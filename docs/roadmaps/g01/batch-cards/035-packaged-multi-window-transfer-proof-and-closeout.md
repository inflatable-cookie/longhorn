# 035 Packaged Multi-window Transfer Proof And Closeout

Status: complete
Owner: Tom
Roadmap: g01.006 batch 5
Governing refs: contracts 001-004 and 009-014; research memos 003 and 010
Depends on: Cards 028-034
Auto-start next card: no

## Objective

Prove direct-container panel transfer and whole-Surface transfer in a packaged
multi-window Tauri app, audit boundaries, and close g01.006.

## Scope

- one minimal packaged proof app
- direct-window and Surface-hosted composition modes
- multi-webview session and lease lifecycle
- explicit-zone and screen-point commits
- same-document panel move
- whole-Surface move and opt-in empty-display provision
- cancellation, expiry, overlap, target loss, stale revision, and replay
- scale and window-boundary cases
- package and authority audit
- contract, roadmap, inventory, and evidence closeout

## Public Behavior

The proof uses minimal local frontend wiring, not reusable Svelte/Poodle
adapters. Every durable success comes from a Rust-authoritative commit.
Failures reconcile without changing the source document.

Packaged evidence must distinguish platform coverage. A macOS proof does not
claim Windows or Linux runtime behavior.

## Out Of Scope

- Loophole or Nucleus migration
- public package publication
- cross-document panel transfer
- copy transfer
- final Svelte/Poodle adapters
- non-Tauri host support

## Steps

1. Add a minimal two-mode packaged proof app.
2. Exercise direct-window panel transfer.
3. Exercise Surface-hosted panel transfer.
4. Exercise whole-Surface window transfer.
5. Exercise opt-in empty-display provision and cleanup.
6. Exercise explicit-zone and screen-point resolution.
7. Inject cancellation, expiry, overlap, stale, replay, and target loss.
8. Exercise scale and outer-window boundary cases.
9. Build the Rust 1.85 locked packaged artifact.
10. Audit dependency, capability, payload, and authority boundaries.
11. Record retained, changed, deferred, and platform-limited behavior.
12. Close Cards 028-035 and g01.006.

## Acceptance Criteria

- Nucleus mode imports no Surface state
- Loophole mode resolves the full host hierarchy
- cross-window panel move commits one registered layout document
- whole-Surface move retains its layout-container binding
- empty-display provision requires explicit policy
- every failed or cancelled transfer preserves exact source state
- overlap never resolves by enumeration order
- expired, stale, replayed, and disappeared targets abort
- packaged app proves real multi-webview handler behavior
- capability and package audits contain no hidden product authority
- docs preserve cross-document, copy, UI, migration, and platform limits

## Evidence Required

- packaged artifact metadata
- two-mode transfer matrix
- failure and cancellation matrix
- screen geometry and scale report
- payload and capability audit
- dependency report
- behavior delta table
- milestone closeout log
- Rust 1.85 and full Effigy QA

## Stop Conditions

- proof requires modifying a consumer repository
- only unpackaged mock behavior can be demonstrated
- failed transfer changes source bytes
- platform geometry cannot be classified truthfully
- contract or package boundary changes during proof

## Next Task

Start Card 036 from the compiled g01.007 runway. Do not start reusable Svelte
or Poodle adapters from this completed card.

## Closeout

Completed 2026-07-29.

- direct and Surface-enabled Rust 1.85 packaged artifacts passed on macOS arm64
- two real renderer webviews exercised snapshot, lease, admission, and commit
- direct panel transfer advanced one registered layout document
- Surface mode advanced panel and Surface documents independently
- explicit-zone and screen-point paths passed
- empty-display policy created, placed, showed, and registered a third window
- cancellation, expiry, overlap, target loss, stale geometry, stale revision,
  and replay retained exact authority bytes
- direct dependency graph contains no Surface package
- payload and capability audits contain no product or broad ambient authority
- Windows, Linux, mixed-scale, multi-display, UI, copy, cross-document, and
  consumer migration work remains explicit

Evidence:
[Packaged Multi-window Transfer Proof And Closeout](../../../logs/2026-07/29-packaged-multi-window-transfer-proof-and-closeout.md).
