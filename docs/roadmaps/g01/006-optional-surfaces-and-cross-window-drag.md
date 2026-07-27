# g01.006 Optional Surfaces And Cross-window Drag

Status: blocked on `g01.003` through `g01.005`  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contracts 002, 009, and 011

## Outcome

Add Loophole's full window-to-Surface composition as a removable module and
prove cross-window transfer for both composition shapes.

## Batches

### 1. Surface host

- identity, labels, presence, lifecycle, ordering, and active Surface
- host preferences, fallback, duplication, move, and close
- Surface-to-layout-container binding

### 2. Transfer protocol

- id-only drag payload plus screen coordinates
- target resolution, eligibility, commit/abort, stale-state handling
- direct-window target for no-Surface consumers
- Surface target for full-hosting consumers

### 3. Packaged proof

- multi-webview/window transfer
- display-scale and window-boundary cases
- cancellation and target disappearance

## Acceptance

- Nucleus imports no Surface state
- Loophole retains full hosting behavior
- cross-window transfer re-resolves current host authority
- failed or cancelled transfer leaves source state intact
