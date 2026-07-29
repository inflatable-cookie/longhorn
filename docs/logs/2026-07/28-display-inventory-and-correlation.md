# Display Inventory And Correlation

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added pure `longhorn-display` over `longhorn-core`
- added persistent known displays and current non-canonical observations
- added namespaced strong, remembered-adapter, and weak evidence
- added mutual one-to-one correlation in strict evidence order
- added explicit available, missing, and unresolved states
- added confidence, winning evidence, candidates, and ambiguity receipts
- added injected allocation for genuinely new displays
- added user/machine label precedence without source erasure
- added explicit forget and deterministic registry serialization
- added versioned arrangement signatures over canonical available displays

## Correlation Policy

Correlation runs strong platform/hardware keys, remembered adapter keys, exact
full bounds plus scale, then unique weak fingerprints. Each tier binds only a
mutually unique known/observed pair. Ambiguity at a stronger tier remains
unresolved; weaker evidence cannot guess through conflicting stronger
evidence.

New identity comes only from `DisplayIdAllocator`. Allocation order is derived
from observation content, not input enumeration. Indistinguishable new
observations remain unresolved instead of receiving order-dependent ids.
Platform ids, Core Graphics UUIDs, adapter keys, weak fingerprints, names, and
geometry never become canonical ids inside the library.

## Donor Delta

Loophole currently permits geometry-only matching when scale differs.
Contract 009 requires exact geometry plus scale. Longhorn therefore does not
carry that fallback: a scale-mismatched observation needs strong, remembered,
or unique weak evidence, otherwise it remains unmatched and enters explicit
new-display handling.

Nucleus name/position/size synthesis is weak evidence. Duplicate synthetic
fingerprints remain ambiguous. Soundcheck's single-display case uses the same
allocator and retained-evidence path without introducing a single-window
special case.

## Arrangement

`longhorn-arrangement-v1` sorts available `DisplayId` values and binds each
id, full bounds, work area, scale thousandths, and main-display status. Missing
or unresolved displays do not enter the active arrangement. The empty
arrangement has one stable versioned value.

## Evidence

- macOS Core Graphics UUID fixture wins as strong evidence without host code
- remembered adapter evidence wins over geometry
- exact geometry/scale and rearranged unique-weak fixtures report their tiers
- duplicate weak fingerprints remain unresolved under input permutation
- missing/reappearing displays preserve id, user label, machine label, and
  remembered evidence
- allocator call count, call order, id retention, and duplicate-id failures pass
- explicit forget removes only the named display
- arrangement permutations and full/work/scale/main/availability changes pass
- registry and signature serde are deterministic and strict
- `longhorn-display` normal dependencies are `longhorn-core` and `serde` only
- Rust 1.85 workspace check passed
- formatting, warnings-denied Clippy, and workspace tests passed
- Effigy QA passed

## Boundary

No Tauri probe, non-macOS hardware discovery, window fallback, placement,
live diff, persistence store, ambiguity UI, TypeScript, Svelte, Poodle,
Surface, product type, or donor write entered the package.

## Posture

`strict-ready`

Card 014 is complete. Card 015 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start Card 015 window placement resolution.
