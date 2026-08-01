# 083 Child-webview Mechanism Packaged Prototype

Status: complete
Owner: Tom
Roadmap: g01.013 batch 3
Governing refs: contracts 001, 003, 010, 012, 013, and 017; research memo 017
Depends on: Card 082
Auto-start next card: no

## Objective

Apply the frozen coordination prototype through an isolated Tauri child-webview
mechanism. Produce packaged macOS evidence for bounds, visibility, focus,
reuse, close, scale, teardown, and remote-content security without promoting a
public adapter.

## Result

The private adapter and packaged macOS arm64 proof pass. Tauri 2.10.3 creates
the controlled remote child through the isolated unstable port. Native 2x
bounds, viewport moves, host resize, zero-size bounds, restore, hide/show
session reuse, renderer-unmount independence, focus request, explicit close,
replacement, stale callback rejection, host destruction, download denial,
navigation denial, and capability confinement are recorded.

Portable child visibility and focus readback remain unknown. The proof host
exposed one 2x monitor, so native scale switching remains explicitly unmet and
unsimulated. Windows and Linux remain unproved. No public package or donor
migration was created.

## Scope

- private child-webview adapter prototype
- packaged product-neutral Tauri proof app
- current unstable `WebviewBuilder` and `Window::add_child` isolation
- consumer-supplied content source, data-store, and capability policy
- listener-before-create observation
- logical child position and size from semantic viewport
- hide/show without renderer-unmount destruction
- explicit focus request and observation
- explicit close and host-window destroy invalidation
- consumer-computed overlay and activity visibility inhibitors
- Nucleus-shaped reuse and close trace
- target support ledger for macOS, Windows, and Linux

## Mechanism Behavior

The adapter creates one child webview for the current island generation. It
maps the desired viewport to child position and size. Renderer lifecycle does
not imply native destruction. Explicit desired absence or host destruction
owns close.

Content URL, navigation, popup, download, data-store, and capability decisions
are constructor inputs. Remote content receives no Longhorn or consumer Tauri
capabilities by default.

## Out Of Scope

- generic browser tabs, browsing history, downloads, or popup policy
- Nucleus cursor workarounds or panel catalogue behavior
- DOM or Poodle overlay discovery in Rust
- isolated native-window or backing-surface adapters
- production package publication
- Nucleus repository migration

## Steps

1. Freeze the Card 082 child-view trace and adapter port.
2. Create an isolated private Tauri proof app and controlled content fixture.
3. Isolate the unstable Tauri feature behind the child adapter.
4. Implement attach, bounds, hide, show, focus, close, and observation.
5. Add listener-first coordination and stale-generation rejection.
6. Exercise inactive, overlay-inhibited, dragged, and restored visibility.
7. Exercise repeated hide/show reuse and explicit terminal close.
8. Exercise host resize, viewport move, zero viewport, and available scale
   changes.
9. Audit remote-content capabilities, navigation policy injection, and labels.
10. Audit the built graph and record per-target support truthfully.

## Acceptance Criteria

- packaged macOS child creation and interaction pass
- native bounds converge to the current desired child-view viewport
- hide/show preserves the controlled child session
- unmount alone does not close the child
- explicit close and host destroy invalidate the generation
- stale measurements and callbacks cannot mutate a replacement child
- overlay behavior arrives only as explicit consumer visibility input
- focus intent and observed focus remain distinct
- remote controlled content has no undeclared Tauri capabilities
- deterministic 1x/2x pure conversion fixtures pass
- native scale-switch evidence is recorded when the host can supply it; absence
  remains an unmet proof, never a simulated platform claim
- the graph omits isolated-window, plugin, GPU, Svelte, and Poodle adapters
- Windows and Linux are proved or recorded unsupported/unproved per target

## Evidence Required

- produced packaged macOS application and scripted evidence trace
- create/reuse/hide/show/focus/close/destroy matrix
- bounds and scale evidence with native observation
- stale generation and rapid replacement fixtures
- controlled remote-content capability audit
- adapter/public-symbol and dependency inventory
- macOS, Windows, and Linux support ledger
- focused Tauri, Rust, renderer, docs, and Effigy checks

## Stop Conditions

- Tauri child-webview behavior requires product navigation authority in core
- remote content gains ambient capabilities
- bounds cannot be observed after apply
- native scale evidence contradicts the Card 082 conversion model
- child lifetime requires renderer mount lifetime
- the adapter depends on another native-content mechanism
- packaged macOS evidence cannot be produced

## Next Task

Execute ready Card 084. Prove isolated native-window negotiation with a
controllable fake native child and disposable helper lifecycle.
