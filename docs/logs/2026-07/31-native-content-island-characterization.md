# Native Content Island Characterization

Date: 2026-07-31  
State: complete research promotion batch

## Outcome

- revalidated current Nucleus child-webview hosting read-only
- revalidated current Soundcheck isolated plugin helper and native `NSView`
  hosting read-only
- revalidated current Jetstream WGPU backing view and transparent-webview
  composition read-only
- recorded lifecycle, geometry, scale, visibility, focus, input, teardown, and
  platform-proof differences
- checked the proposed boundary against current Tauri 2.11.5 webview, window,
  and window-event APIs
- promoted translation memo 017 and contract 017
- selected a shared desired/observed coordination protocol with separate
  mechanisms
- rejected one universal browser/plugin/GPU host implementation

## Decision

The common meaning is one opaque native content island with a host binding,
attach generation, typed client viewport, explicit scale, presence,
visibility, focus intent, input-routing mode, observation, and exact receipts.

The viewport is semantic. Nucleus applies it as child bounds. Soundcheck uses
it as isolated-window content size. Jetstream uses it as render and input
clipping while its backing `NSView` fills the host.

Browser policy stays in Nucleus. Plugin ABI, helper isolation, unsafe unload,
audio/MIDI, and state stay in Soundcheck and Signal. GPU surface creation,
rendering, scene, camera, and input semantics stay in Jetstream.

## Gaps Before Implementation

- compile g01.013 into bounded prototype and proof cards
- prove scale changes instead of relying on startup scale
- preserve `unknown` when portable occlusion evidence does not exist
- build a controllable fake native child for Soundcheck-shaped resize and
  teardown proof
- define independent adapter dependency graphs
- package all three shapes on macOS
- prove or explicitly reject each Windows and Linux mechanism

## Limits

- no donor repository changed
- no production package or public API created
- no cross-platform support claimed
- no browser, plugin, or GPU payload promoted
- no Poodle component or private DOM contract moved into Longhorn

## Validation

- focused Northstar structure and link checks
- live-pointer drift scan
- `git diff --check`

## Posture

`strict-ready`

## Next

Compile g01.013 into a multi-card prototype runway governed by memo 017 and
contract 017. Do not start implementation until the first card passes the
ready rubric.

