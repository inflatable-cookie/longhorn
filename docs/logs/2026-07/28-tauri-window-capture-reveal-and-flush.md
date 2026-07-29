# Tauri Window Capture, Reveal, And Flush

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 020
- translated managed Tauri window events into Card 019 inputs
- connected Card 018 apply evidence before native mutation
- captured outer origin, inner size, normal placement, maximized state, and
  raw current-monitor evidence
- kept unresolved monitor association explicit
- added injected placement staging and bounded acknowledgement flush
- added typed scheduling, capture, persistence, timeout, failure, and
  disconnect receipts
- installed the same listener path for predeclared and dynamic windows
- gated reveal on converged hidden placement plus consumer page readiness
- aggregated stable sorted application-shutdown targets into one flush
- made Card 021 the sole ready lane

## Lifecycle Boundary

The adapter owns native observation and execution only. The sink owns schema,
merge, storage, and home-display adoption. Current-monitor facts never allocate
canonical display identity.

Move and resize use live window scale plus the injected global-plane mapper.
Scale-change events validate their supplied factor. Capture reads outer origin
and inner size. Maximized capture requires retained normal placement from
successful normal capture or pre-mutation move/resize evidence.

Capture, sink, flush, timer, callback, reporter, and reveal calls run outside
coordinator and installed-window locks. A re-entrant sink fixture registers new
apply evidence during staging and completes without deadlock.

## Reveal And Flush

Hidden restore planning suppresses visibility and focus. Reveal occurs only
after a converged move/resize readback and page-ready signal, in either order.
No fixed sleep is present.

Every close flush uses the caller policy's exact bound. Shutdown issues current
forced captures where required, then sends one aggregate sink request. Sink
request failure, sink completion failure, timeout, and disconnect remain
distinct. User close reaches a callback without inferred desired-state
mutation.

Capture or stage failure sends a typed action and resets the Card 019 capture
request. A later explicit flush can retry the same pending generation.

## Evidence

- Loophole programmatic apply suppression and hidden reveal fixture
- Nucleus settle, blur, and one-second close fixture
- Soundcheck two-second sink failure, close, and destroy fixture
- stale generation, capture retry, unresolved display, timeout, and disconnect
  fixtures
- predeclared and dynamic listener installation through one method
- stable sorted aggregate shutdown fixture
- re-entrant sink lock-boundary fixture
- nine focused adapter tests pass
- all 29 `longhorn-tauri-windowing` unit, execution, lifecycle, and observation
  tests pass
- warnings-denied Clippy passes on the current toolchain
- full Effigy QA passes
- god-file scan reports no high or critical findings; 37 warning-level
  findings remain visible

Rust 1.85 cannot resolve the existing locked Tauri dependency graph. ICU now
requires Rust 1.86; current Darling, plist, serde_with, and time require Rust
1.88. Card 022 retains the existing dependency-floor reconciliation gate.

## Boundary

No configuration package, product schema, path policy, fallback file write,
home-display adoption, layout, Surface, renderer protocol, Svelte, Poodle,
consumer mutation, or donor write entered the adapter.

## Posture

`strict-ready`

Card 020 is complete. Card 021 is ready for assembly, capability, teardown,
and full failure-matrix proof.

## Next

Review and explicitly start Card 021. Do not auto-start composition or packaged
proof.
