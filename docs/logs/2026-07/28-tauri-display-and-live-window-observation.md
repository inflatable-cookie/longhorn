# Tauri Display And Live Window Observation

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 017
- added `longhorn-tauri-windowing`
- replaced boolean built-in display facts with strict
  `DisplayBuiltinStatus::{Unknown, BuiltIn, External}`
- added checked finite positive Tauri scale conversion to fixed thousandths
- preserved complete physical display and managed-window facts at the adapter
  edge
- added exact primary-monitor attribution
- added process-local observation metadata with optional injected evidence
- added whole-desktop coordinate mapping and a uniform-scale implementation
- added complete logical display and live-window projection
- made Card 018 the sole ready lane

## Coordinate Policy

`DesktopCoordinateMapper` receives the complete raw physical desktop snapshot.
The built-in mapper accepts only one scale across all displays and managed
windows, then converts with explicit nearest rounding. A mixed-scale snapshot
fails as unavailable unless the consumer supplies one mapper that can establish
a coherent global screen-DIP plane. No monitor origin is divided independently
by its local scale.

## Identity Policy

Tauri monitor names become bounded machine labels only. Default monitor
metadata uses process-local observation ids, unknown built-in status, and no
correlation evidence. Injected providers may supply built-in status and
evidence but cannot allocate canonical `DisplayId`.

Managed webview windows are explicit inputs. Their Tauri labels become opaque
`HostWindowHandle`; optional stable `WindowId` comes only from caller
bookkeeping. Duplicate observation ids, transport handles, or stable window
ids fail before a snapshot is returned.

## Failure Policy

Monitor enumeration, primary lookup, every required window getter, scale
validation, mapping, and projection return typed failures. One failed managed
window prevents the complete batch from returning. Mapped output must contain
exactly one geometry record for every raw display and managed window; missing,
extra, and duplicate records fail.

## Evidence

- Loophole-shaped uniform multi-monitor geometry preserves full/work areas and
  outer/inner window frames
- Nucleus-shaped negative origins survive checked fractional-scale conversion
- Soundcheck-shaped single-monitor state preserves unknown built-in status
- mixed-scale mapping fails without a provider and succeeds through an injected
  complete-plane mapper
- invalid, zero, non-finite, rounded-zero, and overflowing scale or geometry
  cases fail typed
- exact, missing, and ambiguous primary matches are covered
- input permutation and raw/logical serde evidence pass
- Tauri mock runtime proves explicit managed-window inclusion, unmanaged
  exclusion, and fail-complete duplicate rejection
- the lockfile proves Tauri 2.10.3 with `tauri-runtime` 2.10.1
- normal package edges are core, display, windowing, serde, and Tauri only
- Rust 1.85 workspace formatting, warnings-denied Clippy, tests, Effigy docs,
  Northstar, QA, and Doctor pass

Tauri's mock runtime does not implement monitor enumeration. Display probe
semantics use raw-fact fixtures here; packaged monitor proof remains Card 022.

## Boundary

No native mutation, dynamic creation, event listener, debounce, persistence,
layout, Surface, TypeScript, Svelte, Poodle, product type, or donor write
entered the package.

## Posture

`strict-ready`

Card 017 is complete. Card 018 is ready after reassessment against the
implemented physical snapshot, managed-window, and whole-desktop mapper seams.

## Next

Review and explicitly start Card 018. Do not start native mutation
automatically.
