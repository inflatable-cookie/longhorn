# 022 Packaged Window Host Proof And Closeout

Status: complete
Owner: Tom
Roadmap: g01.004 batch 3
Governing refs: contracts 001, 003, 009, and 012; research memo 008
Auto-start next card: no

## Objective

Prove the host in a packaged Tauri application, record the executed platform
matrix, close `g01.004`, and stop at `g01.005`.

## Scope

- minimal `examples/tauri-windowing-proof` packaged application
- one protected predeclared window
- optional dynamic second window
- static product-neutral content and page-ready signal
- persisted proof placement through an injected test sink
- missing/rearranged-display restart scenario
- normal and maximized restore
- hidden-until-positioned reveal
- user move/resize capture
- programmatic apply suppression
- dynamic create/close and protected-slot survival
- close and application-shutdown flush
- executed-platform evidence and unexecuted-platform limits

## Public Behavior

The proof consumes produced package APIs, not donor code. It exposes
single-window and multi-window modes without layout or Surface state.

Runtime evidence names exact OS, Tauri version, display arrangement, and steps.
Unavailable Windows or Linux hosts are recorded as unexecuted; the closeout
does not invent cross-platform runtime proof.

## Out Of Scope

- consumer repository migration
- production UI, Poodle, layout, Surfaces, or cross-window drag
- release publication or registry naming
- non-macOS strong display identity research

## Steps

1. Add the minimal packaged proof app and exact capability policy.
2. Build an installable application artifact.
3. Run protected single-window restore/reveal/capture/flush proof.
4. Run dynamic multi-window create/apply/close proof.
5. Restart with a changed or missing saved display and verify safe placement.
6. Verify maximized state preserves normal placement.
7. Record executed platform facts, failures, and artifact identity.
8. Close `g01.004`, normalize front doors, and stop at `g01.005`.

## Acceptance Criteria

- proof uses the public host package and injected sink
- first paint is hidden until placement and readiness converge
- missing display cannot strand either window
- normal and maximized restart state remain distinct
- programmatic operations produce no user-placement write
- settled user movement persists and restores
- protected primary survives desired-set change
- dynamic window creates and closes with exact capability scope
- close and shutdown flush succeed or expose failure
- evidence names exact artifact and executed platforms
- no layout or Surface dependency enters the proof

## Evidence Required

- packaged artifact path and digest
- exact OS, architecture, Rust, Tauri, and Longhorn versions
- initial, moved, maximized, changed-display, and restart observations
- protected-primary and dynamic-window observations
- sink and flush receipts
- screenshots or structured operator transcript where useful
- Rust 1.85 and full Effigy QA

## Stop Conditions

- only a dev server or workspace unit test is available
- proof requires donor or consumer source
- unavailable platform behavior must be claimed
- failure evidence is missing or close flush is unobservable
- layout, Surface, drag, UI framework, or release work enters scope

## Checkpoint Decision

The available native operator environment is:

- macOS 26.5.2 on arm64
- Xcode 26.6 with macOS SDK 26.5
- Rust and Cargo 1.96.0 for current-toolchain work
- installed `aarch64-apple-darwin`, `x86_64-apple-darwin`, and
  `wasm32-unknown-unknown` Rust targets
- Tauri CLI 2.11.4
- workspace Tauri 2.10.3, Tauri Runtime 2.10.1, and Tauri Utils 2.9.3
- Bun 1.3.14 if proof automation needs it

This is sufficient for packaged macOS arm64 proof. Windows and Linux native
operator hosts are unavailable and remain explicitly unexecuted.

Contract 012 keeps MSRV 1.85. Card 022 must first resolve the locked Tauri
graph to the latest compatible transitive versions, then prove
`cargo +1.85.0 check --workspace --all-targets`. Current blockers are
`serde_with`/Darling, plist/time, and URL/IDNA/ICU dependency paths. Do not
raise `rust-version` silently. If Tauri 2.10.3 cannot produce a sound
Rust-1.85 graph, stop for an explicit Contract 012 compatibility change.

## Completion Evidence

- [Packaged Window Host Proof And Closeout](../../../logs/2026-07/28-packaged-window-host-proof-and-closeout.md)
- packaged macOS arm64 proof executed on Rust 1.85
- Rust 1.85 complete workspace check passed
- guarded reveal, user move/resize, maximized and normal restart, missing-display
  fallback, protected primary, dynamic create/close/re-create, explicit flush,
  and application shutdown flush executed
- Windows and Linux remain explicitly unexecuted

## Next Task

Compile `g01.005` into executable batch cards. Do not start layout
implementation from its milestone summary.
