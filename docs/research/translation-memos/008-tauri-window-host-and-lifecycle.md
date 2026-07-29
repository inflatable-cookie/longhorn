# Tauri Window Host And Lifecycle

Status: promoted
Owner: Tom
Updated: 2026-07-28

## Question

Which Tauri display, native-window apply, event attribution, reveal, and flush
behaviors are proven across donors, and which policies must remain injected?

## Repositories

- `loophole/aura`
- `nucleus/apps/desktop`
- `soundcheck`

Inspection was read-only. Donor worktrees remain consumer authority until a
later migration card names them.

## Loophole Evidence

Aura uses Tauri 2.10.3 and has the complete multi-window host:

- the configured `main` window starts hidden and becomes a protected boot slot
- host bookkeeping retags `main` to the resolved primary `WindowId`
- dynamic windows use consumer URL, title, chrome, minimums, and label policy
- live capture combines outer position with inner size
- apply runs unmaximize, position, size, then optional maximize
- page load and positioned readback jointly gate reveal
- close marks distinguish programmatic from user close
- event capture debounces settled geometry
- the dynamic capability pattern covers `main` and `workspace-*`

Its pure coordinator uses 3-second programmatic suppression, a 5-second user
activity window, and 300-millisecond persistence debounce. Those values are
Loophole policy. User close disabling a desired secondary window is also
product policy.

The current executor ignores individual Tauri call failures and infers stable
ids mechanically from labels. Longhorn must not carry those deltas forward.

## Nucleus Evidence

Nucleus uses Tauri 2.11.2 and proves the no-Surface single-window case:

- `main` starts hidden
- restore reads monitors, resolves and clamps before show
- move, resize, scale-factor change, and focus loss schedule capture
- one worker coalesces writes for 300 milliseconds
- close requests wait up to one second for explicit flush
- an operator restart smoke confirmed size and position restore

Nucleus stores physical outer bounds and has no programmatic-event
suppression. Its current display key combines name, position, and size; this is
weak evidence, not durable identity.

## Soundcheck Evidence

Soundcheck uses Tauri 2.10.3 and proves the minimal settings-integrated host:

- `main` starts hidden, restores into a current work area, then shows
- capture uses outer position and inner size
- move, resize, and scale-factor changes debounce for 300 milliseconds
- close and destroy request a two-second flush
- failed channel or timeout falls back to direct persistence

The fallback write is app-specific and swallows failure. Longhorn instead
returns a typed flush receipt through an injected sink.

## Local Tauri Evidence

The locally resolved Tauri 2.10.3 and 2.11.2 crates both declare Rust 1.77.2,
below Longhorn's Rust 1.85 floor. Tauri exposes runtime-generic monitor and
window probes, logical mutation calls, dynamic webview-window construction,
window events, main-thread dispatch, and a `test` feature with `MockRuntime`.

Tauri `Monitor` values expose physical full bounds, physical work area, name,
and floating scale. They do not expose built-in status. Window values expose
physical outer position, outer size, inner size, maximize, visibility, focus,
and monitor probes.

Tauri and its tao/winit source describe monitor origins in the larger physical
screen area and provide per-monitor scale conversion. They do not guarantee
that dividing every physical origin by that monitor's scale produces one
coherent global logical plane under mixed DPI.

## Promoted Decisions

Promoted into contract 009 and `g01.004`:

- Tauri scale converts to fixed thousandths and physical geometry uses explicit
  nearest rounding for a uniform-scale plane
- mixed-scale global conversion uses an injected whole-desktop mapper and is
  unavailable without one
- built-in status gains an explicit unknown state; Tauri does not fabricate
  external
- ambiguous primary-monitor matching fails typed
- a managed live snapshot is complete or fails
- labels remain transport handles; a host registry owns stable identity
- protected retag is bookkeeping, not label parsing
- dynamic creation delegates product definition to an injected factory
- native apply is ordered, non-transactional, and per-operation receipted
- failed same-window dependencies skip; independent windows continue
- fresh complete readback decides convergence
- apply generation is registered before native mutation
- event attribution uses generation and expected state, not time alone
- timing bounds are caller policy
- capture and flush use an injected persistence sink
- user close reaches consumer policy without inferred desired-state mutation
- reveal requires placement convergence and page readiness

## Compiled Runway

1. Card 017 — Tauri display and complete live-window observation
2. Card 018 — managed identity and native operation execution
3. Card 019 — pure event attribution and settling
4. Card 020 — Tauri capture, reveal, and bounded flush
5. Card 021 — host composition, mock runtime, capabilities, and faults
6. Card 022 — packaged proof and `g01.004` closeout

## Remaining Limits

- non-macOS strong display evidence remains open
- Windows and Linux runtime proof requires matching hosts
- registry names remain unverified until release
- consumer migration remains `g01.014` onward
