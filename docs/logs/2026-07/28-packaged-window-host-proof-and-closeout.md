# Packaged Window Host Proof And Closeout

Date: 2026-07-28
State: complete
Card: 022

## Result

`g01.004` is complete. A packaged macOS arm64 Tauri application exercised the
public `longhorn-tauri-windowing` host with an injected JSON placement sink.
The proof has no layout, Surface, Poodle, or Longhorn configuration dependency.

Windows and Linux native hosts were unavailable. Their runtime behavior remains
unexecuted, not inferred.

## Artifact

- application:
  `target/release/bundle/macos/Longhorn Window Proof.app`
- installable archive:
  `target/release/bundle/macos/Longhorn Window Proof-0.1.0-card022-macos-arm64-rust1.85.zip`
- archive size: 3,120,805 bytes
- archive SHA-256:
  `9956e69f22cad6708dacef8fc3c4d3f9b1710219a1f13396986f2598a874d2b1`
- executable:
  `target/release/longhorn-tauri-windowing-proof`
- executable size: 10,366,112 bytes
- executable SHA-256:
  `2eaa2bc99bd490b9aa65bc53c2d54ee10ee72181ee700ad720778cd135f31222`
- executable format: Mach-O 64-bit arm64
- bundle id: `audio.infiniteloop.longhorn-window-proof`
- bundle version: `0.1.0`

The archive is a local proof artifact, not a signed or notarized release.

## Executed Environment

- macOS 26.5.2 build 25F84, arm64
- Xcode 26.6 build 17F113
- macOS SDK 26.5
- Rust 1.85.0, host `aarch64-apple-darwin`
- Tauri CLI 2.11.4
- Tauri 2.10.3
- Tauri Runtime 2.10.1
- Tauri Utils 2.9.3
- Longhorn 0.1.0 workspace source
- display: one 3600×2338 physical-pixel Retina monitor, scale 2.0
- logical full bounds: 1800×1169 screen DIPs
- logical work area: 1800×1130 screen DIPs

## Rust 1.85 Graph

The locked Tauri graph now uses the latest selected compatible transitive
versions:

- `serde_with` 3.17.0 and Darling 0.21.3
- plist 1.8.0
- time 0.3.45
- `idna_adapter` 1.2.1
- ICU collections, locale, normalizer, and provider 2.1.1
- ICU properties 2.1.2

Seven Longhorn let-chain expressions were rewritten as equivalent nested
conditionals. No MSRV change or compatibility fallback was added.

`cargo +1.85.0 check --workspace --all-targets` passes.

## Operator Transcript

The proof sink wrote:

- placement state:
  `~/Library/Application Support/audio.infiniteloop.longhorn-window-proof/placement-state.json`
- structured JSONL:
  `~/Library/Application Support/audio.infiniteloop.longhorn-window-proof/operator-transcript.jsonl`

Selected executed observations:

| Case | Evidence |
| --- | --- |
| guarded first reveal | native config started hidden; page readiness first reported `placement_ready: false`; the next-turn readback converged; reveal reported `revealed` |
| programmatic suppression | initial `MoveResize` registered generation 1; the resulting native move and resize reports were ignored as `programmatic_apply`; no user placement was staged |
| settled user resize | native corner resize settled and staged 780×570, then 700×510, with successful debounce flushes |
| settled user move | macOS Window → Center settled at outer origin 510,319 and flushed |
| normal/maximized separation | 780×570 normal placement was retained while maximized; packaged relaunch restored maximized; unmaximize returned to the retained normal geometry |
| protected primary | omitting `main` from the desired set produced an empty converged apply; the native main survived and was restored to desired bookkeeping |
| dynamic window | `workspace` created hidden, converged on the next event-loop turn, revealed after page readiness, closed, and reduced installed listener count from two to one |
| repeated dynamic window | the same `workspace` slot created and closed again after the close-bookkeeping fix |
| missing saved display | saved home `proof-display:missing` and origin 50000,50000 resolved by `main_display` to available `proof-display:0` at 900,480, size 900×650 |
| explicit flush | window-scope `explicit` request completed `succeeded` within 1500 ms |
| shutdown flush | application-shutdown aggregate completed `succeeded`, then teardown deactivated one listener |

The missing-display result is fully inside the 1800×1130 work area. The
configured missing home remained evidence; it was not rewritten as the current
display.

## Proof-Driven Fixes

Native execution exposed four failures before the final artifact:

1. A 16-bit generated PNG supplied twice the expected RGBA byte count. The
   source icon is now emitted as 8-bit RGBA.
2. Immediate macOS readback can precede delivery of a successful move. Initial
   and dynamic hidden restore now retry on later event-loop turns and never
   reveal before convergence.
3. The static page readiness script required module execution. It now runs as
   `type="module"`.
4. Successful dynamic close retained the dead transport handle in
   `ManagedWindowRegistry`. Close now removes registry bookkeeping after native
   success. The execution suite asserts stale windows are absent, and packaged
   create-close-create-close now succeeds.

## Validation

- Rust 1.85 workspace check: passed
- scoped native execution tests: 9 passed
- packaged build through `effigy proof-windowing-build`: passed
- full Effigy QA: passed

## Limits

- Windows native runtime: unexecuted
- Linux native runtime: unexecuted
- multi-display rearrangement: represented by an explicit missing saved
  display on the executed single-display host
- mixed-scale desktop mapper: not executed
- code signing, notarization, registry publication, and consumer migration:
  out of scope
