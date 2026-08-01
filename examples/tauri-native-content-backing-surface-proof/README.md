# Tauri native-content backing-surface proof

Packaged macOS proof for `longhorn-native-content-backing-surface`.

The production adapter owns generation checks, exact receipts, full-host
storage evidence, clipping, renderer lifecycle, and physical input admission.
The proof-only runtime owns the real AppKit `NSView`, deterministic renderer,
and raw native pointers. Semantic input remains consumer code.

```sh
effigy build:native-content-backing-surface-production-proof
LONGHORN_BACKING_SURFACE_PRODUCTION_EVIDENCE_DIR="$PWD/examples/tauri-native-content-backing-surface-proof/evidence/runs/2026-08-01-macos-arm64" \
  target/release/bundle/macos/Longhorn\ Backing\ Surface\ Production\ Proof.app/Contents/MacOS/longhorn-native-content-backing-surface-proof
effigy verify:native-content-backing-surface-production-proof
```

The committed run is macOS arm64. A live scale transition is recorded as
unmet when the available monitors expose only one scale. Windows and Linux are
unsupported until a target-specific runtime and packaged proof exist.
