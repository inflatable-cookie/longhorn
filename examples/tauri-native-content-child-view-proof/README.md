# Tauri native-content child-view proof

Packaged macOS proof for `longhorn-tauri-native-content-child-view`.

The app starts a controlled loopback HTTP page, attaches it as a native child,
and records production adapter behavior. It covers physical bounds, readiness,
hide/show reuse, renderer unmount, focus request, close and replacement,
teardown, host destruction, policy-admitted retained navigation, same-URL
idempotence, deterministic 1x/2x conversion, capability isolation, and the
available native scale environment.

```sh
effigy build:native-content-child-view-proof
LONGHORN_CHILD_VIEW_EVIDENCE_DIR=/absolute/output/path \
  target/release/bundle/macos/Longhorn\ Native\ Content\ Child\ View\ Proof.app/Contents/MacOS/longhorn-native-content-child-view-proof
effigy verify:native-content-child-view-proof
```

Windows and Linux remain unproved. Build output is not committed; the final
report, transcript, inventory, and target ledger are.
