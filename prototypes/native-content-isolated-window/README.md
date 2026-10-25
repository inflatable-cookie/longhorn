# Private Isolated Native-window Mechanism Prototype

Card 084 evidence only. This nested workspace is non-publishable, does not
join Longhorn's root workspace, and creates no production adapter authority.

## Shape

- `adapter`: the tested isolated-window plan executor and same-binary helper
  process runtime
- `proof-app`: a packaged macOS application whose helper owns a controlled
  native window and fake `NSView` child
- `evidence/target-support.json`: explicit platform support ledger
- `evidence/inventory.json`: dependency, native-handle, capability, placement,
  and bundle inventory
- `evidence/runs/2026-07-31-macos-arm64`: clean packaged transcript and report

The adapter depends on the private Card 082 coordination prototype. Its graph
contains no plugin SDK, Signal, child-webview, backing-surface, GPU, Svelte, or
Poodle edge.

## Authority

The desired viewport supplies physical content size. Child size requests stay
generation- and revision-bound proposals until consumer policy accepts,
constrains, or rejects them. Outer placement is supplied as process launch
arguments by the proof consumer through `longhorn-windowing`; it is absent from
the adapter port.

The helper uses the packaged executable with `--longhorn-isolated-helper` and
a newline-delimited JSON control channel. It owns the native window and fake
child. Teardown reports cooperative completion, unresolved timeout, or owner
process termination. No path loads or unloads third-party code.

## Native Boundary

Raw AppKit FFI is confined to `proof-app/src-tauri/src/native_macos.rs`. It
inserts a real fake `NSView` beneath the helper window and returns only a
boolean attachment observation. No pointer enters the adapter or wire format.
The hidden local controller has an empty Tauri permission set.

## Validation

The prototype is its own workspace, so Oxide is entered from that directory
rather than by a Cargo `--manifest-path`. From the repository root:

```sh
(cd prototypes/native-content-isolated-window && \
  RUSTUP_TOOLCHAIN=1.85.0 oxide test --workspace --offline && \
  oxide clippy --workspace --all-targets --offline -- -D warnings && \
  oxide fmt --all --check)
```

The packaged app builds through one `exec` lease over the proof app's workspace:

```sh
cd prototypes/native-content-isolated-window/proof-app/src-tauri
RUSTUP_TOOLCHAIN=1.85.0 oxide exec -- cargo tauri build --bundles app --ci
```

The nested lock pins Tauri 2.10.3 and Rust-1.85-compatible transitive
versions. The packaged macOS run passes every recorded check. Windows and
Linux are explicitly unsupported by this proof.
