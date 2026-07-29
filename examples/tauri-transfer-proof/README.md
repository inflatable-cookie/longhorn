# Tauri Transfer Proof

Minimal packaged multi-window proof for `longhorn-tauri-transfer`.

The default build is the direct-window Nucleus shape and has no Surface
dependency. The `surface-mode` feature adds the optional Loophole hierarchy.
Both modes use two real Tauri webviews. The target renderer publishes its own
lease; the source renderer admits and commits through the public commands.
Backend closeout reloads the registered document before passing the run.

The Surface build also moves two whole Surfaces: one into the existing target
window and one through explicit empty-display policy into a newly provisioned
window. Layout-container bindings survive both moves.

```sh
effigy proof-transfer-check-direct
effigy proof-transfer-check-surface
effigy proof-transfer-build-direct
effigy proof-transfer-build-surface
```

Set `LONGHORN_TRANSFER_PROOF_AUTORUN=1` to execute on launch and exit with the
proof result. `LONGHORN_TRANSFER_PROOF_OUTPUT` selects the structured JSONL and
final-report directory.

The packaged matrix covers cancellation, expiry, overlapping windows, target
loss, stale geometry, stale layout revision, replay, scale, and half-open
outer-window boundaries. It is macOS evidence only.
