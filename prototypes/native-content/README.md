# Private Native-content Coordination Prototype

Card 082 evidence only. This nested workspace is non-publishable, is not a
root workspace member, and is not a consumer dependency.

## Question

Can one pure desired/observed coordination vocabulary represent all three
native-content shapes without sharing their host implementation?

| Shape | Desired viewport effect | Product authority kept outside |
| --- | --- | --- |
| Nucleus child webview | native child bounds | URL, navigation, data store, capabilities |
| Soundcheck isolated window | content-area size | plugin ABI, helper policy, audio/MIDI, unload |
| Jetstream backing surface | render and input clip | surface, renderer, scene, camera, input semantics |

## Boundary

- uses `longhorn-core` typed client/physical geometry, scale, and window id
- owns private island/kind identity, revisions, attach generations, desired
  and observed state, pure plans, receipts, and content-size proposals
- carries no arbitrary product payload or raw native handle
- imports no Tauri, browser engine, plugin SDK, GPU stack, Svelte, or Poodle
- cannot satisfy packaged proof or authorize production package promotion

## Validation

```sh
cargo +1.85.0 test --manifest-path prototypes/native-content/Cargo.toml --offline
cargo clippy --manifest-path prototypes/native-content/Cargo.toml --all-targets --offline -- -D warnings
cargo fmt --manifest-path prototypes/native-content/Cargo.toml --all --check
```
