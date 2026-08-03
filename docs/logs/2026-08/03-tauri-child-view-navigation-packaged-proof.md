# Tauri Child-view Navigation Packaged Proof

Date: 2026-08-03
Card: 133
Roadmap: g01.020

## Result

The production Tauri adapter now reads and navigates through the retained
private webview. A packaged macOS child changed from the controlled `/proof`
document to `/navigated` without attach, close, or generation change.

## Exact Evidence

- submitted receipt: generation 1, previous `/proof`, requested `/navigated`
- page-load start and finish: one each for the submitted navigation
- repeat exact URL: `unchanged`, zero further native load, one server request
- denied external URL: typed policy denial, retained generation 1
- packaged checks: 8 pass, 1 observed unknown, 1 environment unmet
- unknown: portable focus and visibility readback
- unmet: live scale switch on the one-monitor 2x host
- app bundle: 9208 KiB
- executable SHA-256:
  `533ecec8cb350ff0234df1004a5c3e4570c941dfb8ca7c3c43eb7af71df15576`

## Validation

- `effigy qa:native-content-child-view`
- `effigy qa:native-content-child-view-proof`
- `effigy check:native-content-bindings`

## Next

Card 134 proves the produced source artifact and records consumer resume gates.
