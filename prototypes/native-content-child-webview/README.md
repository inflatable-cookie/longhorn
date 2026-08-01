# Private Child-webview Mechanism Prototype

Card 083 evidence only. This nested workspace is non-publishable, does not
join Longhorn's root workspace, and creates no production adapter authority.

## Shape

- `adapter`: the tested child-only plan executor and its isolated unstable
  Tauri `WebviewBuilder`/`Window::add_child` port
- `proof-app`: a packaged macOS application with a controlled remote HTTP
  fixture and an automatic evidence trace
- `evidence/target-support.json`: explicit platform support ledger
- `evidence/inventory.json`: dependency, private-symbol, capability, and bundle
  inventory
- `evidence/runs/2026-07-31-macos-arm64`: clean packaged transcript and report

The adapter depends on the private Card 082 coordination prototype. Its graph
contains no isolated-window, plugin, GPU, Svelte, or Poodle edge.

## Security

Content source, navigation admission, data-store identity, popup denial,
download denial, and remote-capability posture are explicit constructor
inputs. The packaged remote child has no Tauri capability. The local hidden
controller also has an empty permission set.

## Validation

```sh
cargo +1.85.0 test --manifest-path prototypes/native-content-child-webview/Cargo.toml --offline
cargo clippy --manifest-path prototypes/native-content-child-webview/Cargo.toml --workspace --all-targets --offline -- -D warnings
cargo fmt --manifest-path prototypes/native-content-child-webview/Cargo.toml --all --check
cd prototypes/native-content-child-webview/proof-app/src-tauri
cargo +1.85.0 tauri build --bundles app --ci
```

The nested lock pins Tauri's mutually compatible 2.10 runtime family and
Rust-1.85-compatible transitive versions. The packaged run reports native
scale switching as unmet because the proof host exposed one 2x monitor. It
does not simulate a platform claim.
