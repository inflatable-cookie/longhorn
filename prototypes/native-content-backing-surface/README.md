# Private Backing-surface Mechanism Prototype

Card 085 evidence only. This nested workspace is non-publishable, does not
join Longhorn's root workspace, and creates no production adapter authority.

## Shape

- `adapter`: a backing-only Card 082 plan executor with full-host storage,
  viewport clips, input admission, generation checks, and detach receipts
- `proof-app`: a packaged macOS Tauri application with a controlled AppKit
  backing view and consumer-owned deterministic renderer
- `evidence/target-support.json`: explicit platform support ledger
- `evidence/inventory.json`: native boundary, dependency, payload, Poodle, and
  bundle inventory
- `evidence/runs/2026-07-31-macos-arm64`: packaged transcript and report

The adapter depends only on the private Card 082 coordination prototype,
`longhorn-core`, and Serde. It imports no Tauri, AppKit, child-webview,
isolated-window, renderer, WGPU, scene, Svelte, or Poodle edge.

## Ownership

The native root view remains full-host. Desired viewport changes move and
resize only the consumer render/output clip. The consumer owns rendering and
semantic input payloads. The adapter sees a physical point only long enough
to admit or reject it against current presentation, clip, host-focus, and
input-routing evidence.

Raw AppKit handles and unsafe code are confined to the proof app's
`native_macos.rs`. The selected controlled view supports reversible detach;
host destruction invalidates callback authority before removal and release.

## Validation

```sh
cargo +1.85.0 test --manifest-path prototypes/native-content-backing-surface/Cargo.toml --workspace --offline
cargo clippy --manifest-path prototypes/native-content-backing-surface/Cargo.toml --workspace --all-targets --offline -- -D warnings
cargo fmt --manifest-path prototypes/native-content-backing-surface/Cargo.toml --all --check
cd prototypes/native-content-backing-surface/proof-app/src-tauri
cargo +1.85.0 tauri build --bundles app --ci
```

The nested lock pins Tauri's compatible 2.10 runtime family and Rust
1.85-compatible transitive versions. The packaged run reports native scale
switching as unmet because the host exposed one 2x monitor. It does not
simulate a platform claim.
