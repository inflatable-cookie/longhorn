# Packaged Multi-window Transfer Proof And Closeout

Date: 2026-07-29
State: complete
Card: 035

## Result

`g01.006` is complete. One minimal Tauri application produced separate direct
and Surface-enabled packaged macOS arm64 artifacts. Both ran two real renderer
webviews through the public transfer commands. The Surface build also created,
placed, showed, and registered a third native window from an empty-display
screen-point commit.

Windows and Linux native hosts were unavailable. Their runtime behavior
remains unexecuted.

## Artifacts

Direct mode:

- application:
  `target/release/bundle/macos/Longhorn Transfer Proof.app`
- archive:
  `target/release/bundle/macos/Longhorn Transfer Proof-0.1.0-card035-direct-macos-arm64-rust1.85.zip`
- archive size: 3,318,638 bytes
- archive SHA-256:
  `018f335df9255f5c5a4ed8f244a7e3fe89ea1dd32aab0ee8720d83cb669927b5`
- executable size: 10,821,056 bytes
- executable SHA-256:
  `bd9b70521031fb5a962f6ff126d83dc5b4c0fefafd5b7d6375437424072bafea`
- final report:
  `target/card035-direct-final.ZFcUR0/final-report.json`

Surface mode:

- archive:
  `target/release/bundle/macos/Longhorn Transfer Proof-0.1.0-card035-surface-macos-arm64-rust1.85.zip`
- archive size: 3,559,755 bytes
- archive SHA-256:
  `8f82e7c889d2f3516b67d2bf9e839c17b688768fe4786f484a8b0748f258ddb6`
- executable size: 11,479,872 bytes
- executable SHA-256:
  `07f00a1861eddd86f2d74a467d6a9fd87ef98a13188f37bf8909b982d319f5b2`
- final report:
  `target/card035-surface-final.0vEp3F/final-report.json`

Both executables are Mach-O 64-bit arm64. The bundle id is
`audio.example.longhorn-transfer-proof`; version is `0.1.0`. These are
local unsigned proof artifacts, not releases.

## Executed Environment

- macOS 26.5.2 build 25F84, arm64
- Rust 1.85.0, host `aarch64-apple-darwin`
- Tauri CLI 2.11.4
- Tauri 2.10.3
- Longhorn 0.1.0 workspace source
- one 1800×1169 logical Retina display
- observed scale factor: 2.0

## Two-mode Matrix

| Mode | Panel host | Panel result | Surface result | Native windows |
| --- | --- | --- | --- | --- |
| direct | `direct_window` | layout revision 7→8, source to target | absent | source + target |
| Surface | `surface_container` | layout revision 7→8, source to target | existing target 7→8; provisioned target 8→9 | source + target + provisioned |

Direct mode has no `longhorn-surface-transfer`, `longhorn-surfaces`, or
`longhorn-surfaces-config` dependency. The `surface-mode` feature adds those
three optional packages. Both modes use the same transfer coordinator, panel
adapter, generated protocol, Tauri runtime projection, and registered layout
commit.

The two Surface moves retained:

- `surface:source` → `container:source`
- `surface:second` → `container:target`

The provisioned window was visible after commit. Fresh managed readback
reported outer origin 660,404 and content size 480×360 screen DIPs, exactly
matching requested placement.

## Failure Matrix

Every case used an isolated registered layout domain. The proof read the exact
authority bytes immediately before and after the failed attempt.

| Case | Stable rejection | Consumed | Exact bytes retained |
| --- | --- | --- | --- |
| cancellation | `session_cancelled` | no | yes |
| expiry | `session_expired` | no | yes |
| overlapping windows | `ambiguous_window` | yes | yes |
| target loss | `target_window_missing` | yes | yes |
| stale outer geometry | `stale_window_geometry` | yes | yes |
| stale layout revision | `stale_layout_revision` | yes | yes |
| replay after success | `session_replayed` | no | yes, against post-success authority |

Overlap was run with two containing live-window bounds. It rejected before
lease selection; enumeration order supplied no fallback.

## Geometry

The packaged host projected both native windows into global screen DIPs:

| Window | Outer/content bounds | Scale | Boundary |
| --- | --- | --- | --- |
| source | 40,80 560×500 | 2.0 | right and bottom edges excluded |
| target | 660,80 560×500 | 2.0 | right and bottom edges excluded |

Content bounds were contained by outer-frame bounds. The empty-display point
1776,1145 was inside the primary display and outside both managed windows.
Only the Surface build enabled the explicit provision policy for that point.

Mixed-scale and multi-display geometry remain unexecuted.

## Payload And Capability Audit

Renderer drag payloads contained only:

```json
{"protocol_version":1,"session_id":"<128-bit process-local id>"}
```

No subject, document, revision, binding, window, or product state crossed in
the drag payload. Start requests named a panel or Surface id; the host resolved
current caller window, client epoch, source placement, binding, document, and
revision. Terminal explicit-zone and screen-point selectors were untrusted
hints. Fresh managed-window readback and current leases determined the target.
Rust adapters performed the only durable commits.

The application capability covers `source`, `target`, and `provisioned` with
only `core:default`. Longhorn commands are application-owned invoke handlers;
no filesystem, shell, network, dialog, or broad Tauri plugin permission was
added.

## Proof-driven Corrections

1. Contract 011 requires terminal screen coordinates so whole-Surface transfer
   can distinguish empty display from content zones. The private Card 034
   protocol had narrowed this to a caller-content `ClientPoint`. It now uses
   an untrusted global `ScreenPoint`; checked client projection remains the
   lease-publication boundary. Generated TypeScript and fixtures were
   regenerated with no compatibility shim.
2. The proof's separate deterministic panel and Surface allocators initially
   generated the same session id against one shared coordinator. Their proof
   entropy ranges are now disjoint. Production callers still supply the
   allocator contract.
3. Autorun failures now persist a structured failed report and exit nonzero
   instead of leaving a diagnostic window open.

## Behavior Delta

| State | Behavior |
| --- | --- |
| retained | optional Surface hierarchy; no-Surface direct composition; one-document panel move; Surface layout binding; complete leases; host-authoritative commit |
| changed | terminal point selector aligned to contracted global screen DIPs; proof allocator ranges made collision-free |
| deferred | cross-document panel transaction; copy; reusable Svelte/Poodle drag UI; consumer migrations; registry publication |
| platform-limited | Windows, Linux, mixed-scale, multi-display, signing, and notarization unexecuted |

Provision failure cleanup and unresolved-cleanup evidence remain covered by
the deterministic Card 033 suite. A successful provision becomes ordinary
managed application state; this packaged run retained it until normal
application teardown.

## Validation

- Rust 1.85 direct and Surface checks: passed
- direct and Surface packaged builds: passed
- direct and Surface packaged autoruns: passed
- dependency, payload, capability, geometry, and authority audits: passed
- full Effigy QA: passed

