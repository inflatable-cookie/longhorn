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

The prototype is its own workspace, so Oxide is entered from that directory
rather than by a Cargo `--manifest-path`. From the repository root:

```sh
(cd prototypes/native-content && \
  RUSTUP_TOOLCHAIN=1.85.0 oxide test --offline && \
  oxide clippy --all-targets --offline -- -D warnings && \
  oxide fmt --all --check)
```
