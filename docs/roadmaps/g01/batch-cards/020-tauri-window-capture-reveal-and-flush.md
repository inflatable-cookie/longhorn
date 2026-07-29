# 020 Tauri Window Capture, Reveal, And Flush

Status: complete
Owner: Tom
Roadmap: g01.004 batch 2
Governing refs: contracts 001, 004, and 009; research memos 005 and 008
Auto-start next card: no

## Objective

Bind Tauri window events to settled placement capture, injected persistence,
readiness reveal, and bounded close/shutdown flush.

## Scope

- Tauri `WindowEvent` translation into Card 019 inputs
- complete settled geometry recapture after debounce
- current monitor observation without fabricated `DisplayId`
- captured normal placement separate from maximized state
- injected placement sink and flush interface
- typed schedule, persist, timeout, and sink-failure receipts
- predeclared and dynamic window listener installation
- hidden restore/apply plus page-readiness reveal gate
- focus-loss capture
- close-request and application-shutdown flush
- user-close callback to consumer policy

## Public Behavior

The adapter captures stable `WindowId`, host facts, normal placement, maximized
state, and current monitor evidence. Unresolved display association remains
explicit. The injected sink owns schema, storage, merge, and home-adoption
policy.

Reveal requires both successful placement readback and a consumer page-ready
signal. Close and shutdown wait only for an explicit bound and return a
receipt. Timeout or sink failure is observable. There is no fallback direct
file write.

## Out Of Scope

- product settings schemas or paths
- automatic home-display adoption
- renderer IPC protocol
- user-close disable/delete behavior
- layout, Surfaces, Poodle, or donor writes

## Steps

1. Define captured-placement, display-association, sink, and flush contracts.
2. Translate Tauri events and execute Card 019 directives.
3. Recapture complete settled facts at the current generation.
4. Schedule and force sink writes without holding host registry locks.
5. Install listeners for boot and dynamically created windows.
6. Gate reveal on placement convergence and page readiness.
7. Flush on close and application shutdown with bounded acknowledgement.
8. Return user-close and failure receipts to consumer policy.

## Acceptance Criteria

- capture uses outer origin and inner size, not outer extent
- maximized capture retains a separate normal placement
- unresolved monitor identity remains explicit
- programmatic events produce no sink mutation
- stale debounce generation cannot publish
- sink calls occur outside registry/coordinator locks
- close and shutdown have bounded waits
- timeout and sink failure are typed and inspectable
- hidden windows cannot reveal before placement and page readiness
- user close reaches consumer policy without implicit desired-state mutation

## Evidence Required

- Loophole multi-window apply suppression and reveal fixtures
- Nucleus move/resize/scale/blur/one-second-close fixture
- Soundcheck move/resize/scale/close/destroy/two-second-flush fixture
- sink failure, timeout, stale generation, and unresolved-display fixtures
- predeclared and dynamic listener lifetime tests
- clean shutdown aggregate flush fixture
- Rust 1.85 and full Effigy QA

## Stop Conditions

- the adapter must depend on `longhorn-config`
- a timeout or persistence error must be swallowed
- page readiness must become a fixed sleep
- unresolved monitor evidence must become a canonical id
- product close policy, layout, Surface, or UI state enters scope

## Readiness

Card 019 now supplies pure timestamped inputs, exact apply attribution,
generation-coalesced capture and debounce, bounded flush directives,
consumer-owned user-close reporting, and terminal forget behavior. Card 020
owns only Tauri translation, complete capture, injected sink execution,
readiness reveal, and typed adapter receipts.

## Outcome

`longhorn-tauri-windowing` now translates managed Tauri move, resize, scale,
blur, close, and destroy events into the pure lifecycle coordinator. Card 018
registers exact apply evidence with the host before every native mutation.
Matching programmatic events therefore stop before capture or sink mutation.

Settled and blur capture use outer origin plus inner size. Maximized capture
requires retained normal placement. Current-monitor results preserve raw
physical bounds, work area, label, and scale without allocating `DisplayId`;
no current monitor remains explicitly unresolved.

The injected sink stages schema-opaque placement and returns an acknowledgement
ticket for bounded flush. Schedule rejection, capture failure, stage failure,
flush request failure, sink failure, timeout, and disconnect remain typed.
Capture or staging failure resets Card 019 state so an explicit later flush can
retry instead of leaving a generation permanently requested.

Predeclared and dynamic windows use the same listener installation method.
Destroy flushes then forgets the installed slot. Application shutdown gathers
stable sorted targets into one bounded aggregate request. Sink, callback,
capture, reveal, and scheduling calls run outside coordinator and installed
window locks.

Reveal has no sleep. Windows restore through the hidden planning mode and show
only after both converged move/resize readback and consumer page readiness.

Nine adapter fixtures cover Loophole apply suppression, Nucleus
move/settle/blur/one-second close, Soundcheck two-second failure and destroy,
stale generation, retry, unresolved display, timeout, disconnect, dynamic
aggregate shutdown, reveal ordering, and re-entrant sink execution. Package
tests and warnings-denied Clippy pass on the current toolchain.

The declared Rust 1.85 check remains blocked before compilation by the existing
locked Tauri dependency floor: ICU requires 1.86 and current Darling, plist,
serde_with, and time require 1.88. Card 022 already owns dependency-floor
reconciliation and packaged distribution proof.

## Next Task

Card 021 is ready for complete host composition, fault injection, capability
examples, and mock-runtime proof. Review it before starting; do not enter the
packaged Card 022 lane automatically.
