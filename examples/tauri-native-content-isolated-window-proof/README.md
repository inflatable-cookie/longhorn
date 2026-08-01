# Tauri native-content isolated-window proof

Packaged macOS proof for `longhorn-native-content-isolated-window`.

The controller uses the production adapter over the frozen Card 084
same-binary process fixture. The helper creates a real controlled `NSView`
child, exchanges generation- and correlation-bound messages, and remains
disposable. Raw AppKit pointers stay in the proof-only platform module.

```sh
effigy build:native-content-isolated-window-proof
LONGHORN_ISOLATED_WINDOW_EVIDENCE_DIR="$PWD/examples/tauri-native-content-isolated-window-proof/evidence/runs/2026-08-01-macos-arm64" \
  target/release/bundle/macos/Longhorn\ Isolated\ Window\ Proof.app/Contents/MacOS/longhorn-native-content-isolated-window-proof
effigy verify:native-content-isolated-window-proof
```

The committed run is macOS 26.5.2 arm64. Windows and Linux are unsupported.
