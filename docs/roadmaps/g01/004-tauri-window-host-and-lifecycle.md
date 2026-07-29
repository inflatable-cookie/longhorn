# g01.004 Tauri Window Host And Lifecycle

Status: complete
Owner: Tom
Updated: 2026-07-28
Governing refs: contracts 001, 003, 009, 010, and 012; research memos 003 and
008

## Outcome

Apply pure window plans to Tauri 2 webview windows, observe displays and live
geometry without identity substitution, and capture durable settled placement
without feedback loops.

## Goals

- [x] Tauri monitor and window facts convert through typed physical and
  screen-DIP geometry.
- [x] Managed host identity remains explicit across a protected boot window and
  dynamic windows.
- [x] Every planned operation produces typed non-transactional apply evidence.
- [x] Programmatic native events cannot become durable user-placement writes.
- [x] Restore, readiness reveal, close, and shutdown have bounded inspectable
  lifecycle receipts.
- [x] One-window and multi-window compositions work without layout or Surface
  dependencies.

## Execution Plan

- [x] Batch 1 — host observation and apply:
  - [Card 017](batch-cards/017-tauri-display-and-live-window-observation.md)
    adds the narrow Tauri 2 package, checked host conversion, and complete
    managed snapshots.
  - [Card 018](batch-cards/018-tauri-window-operation-execution.md) adds
    explicit identity bookkeeping, injected creation, ordered native execution,
    per-operation receipts, and convergence readback.
- [x] Batch 2 — event and persistence lifecycle:
  - [Card 019](batch-cards/019-window-event-attribution-and-settling.md) adds the
    pure apply/user attribution and settling coordinator.
  - [Card 020](batch-cards/020-tauri-window-capture-reveal-and-flush.md) binds
    Tauri events to settled capture, injected persistence, readiness reveal,
    and bounded close/shutdown flush.
- [x] Batch 3 — composition and packaged proof:
  - [Card 021](batch-cards/021-tauri-window-host-composition-and-mock-proof.md)
    adds mock-runtime assembly, capability guidance, failure injection, and
    donor-shaped composition fixtures. Complete.
  - [Card 022](batch-cards/022-packaged-window-host-proof-and-closeout.md) adds a
    packaged native proof, changed-display restart evidence, and milestone
    closeout.

## Acceptance Criteria

- [x] Host scale and geometry conversion is checked, explicit, and Rust
  1.85-compatible.
- [x] Mixed-scale global coordinates use declared platform evidence, not
  per-monitor origin division.
- [x] Incomplete managed-window observation cannot trigger duplicate creation.
- [x] Stable `WindowId` never comes from a label, monitor name, or enumeration
  index.
- [x] Consumer creation policy supplies URL, title, chrome, minimum size, and
  capability shape.
- [x] Retag/create precede geometry; geometry precedes reveal/focus; close runs
  last.
- [x] Partial native failure is visible and independent windows still progress.
- [x] Fresh readback, not intended operations, determines convergence.
- [x] Apply generations are registered before native mutation.
- [x] User move, resize, scale, blur, close, and destroy paths are covered.
- [x] Debounce, attribution, settle, and flush bounds are explicit inputs.
- [x] Pure programmatic apply and close attribution cannot emit durable user
  state directives.
- [x] Clean close and shutdown flush; timeout and sink failure are inspectable.
- [x] Windows remain hidden until placement and page readiness both complete.
- [x] Tauri capability examples cover protected and dynamic window patterns
  without broad renderer authority.
- [x] Nucleus-shaped single-window and Loophole-shaped multi-window fixtures
  use the same host package without layout or Surfaces.
- [x] A packaged app proves restore, reveal, move/resize, maximize, restart,
  protected-primary reuse, dynamic creation, and close flush.

## Lane Runway

- Generation goal: supply the native host required before layout, Surface, UI,
  and consumer migration lanes can depend on window lifecycle.
- Active now: none.
- Delivered runway: Cards 017-022.
- Packaged proof: macOS arm64 executed; Windows and Linux unexecuted.
- Next planning checkpoint: compile `g01.005`; do not infer layout authority
  from its milestone summary.

## Planning Gaps

- Non-macOS strong display evidence remains a separate packaged-adapter research
  item. Tauri observation may supply weak evidence without fabricating durable
  identity.
- Tauri does not guarantee a coherent global logical plane by dividing every
  physical monitor origin by that monitor's scale. Card 017 uses an injected
  mapper for mixed-scale arrangements and reports unavailable without one.
- Windows and Linux runtime proof requires matching operator or CI hosts. Card
  022 records executed platforms instead of claiming unavailable evidence.
- The complete workspace checks on Rust 1.85. Card 022 selected compatible
  Tauri-side transitive versions without raising Contract 012's floor.
- Registry names remain working names until the release milestone.

## Current Gate

`g01.004` is complete. The executed evidence is recorded in
[Packaged Window Host Proof And Closeout](../../logs/2026-07/28-packaged-window-host-proof-and-closeout.md).
Stop at the `g01.005` planning gate.
