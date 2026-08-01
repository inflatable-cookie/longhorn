# History System Artifact Proof

Two disposable consumers prove the public linear history slice from produced
artifacts.

- `minimal` uses pure Rust and framework-neutral TypeScript.
- `loophole` adds consumer-owned product policy, model transaction,
  persistence, journal recovery, Tauri, Svelte, and Poodle.

The verifier copies these roots into random temporary directories. Rust
consumers compile against private package inventories. TypeScript consumers
install packed tarballs. No proof may resolve Longhorn or Poodle sibling
source.

Each Rust proof emits a renderer fixture and the expected public trace. The
matching TypeScript proof drives `HistoryController` against that fixture and
must emit the same trace.
