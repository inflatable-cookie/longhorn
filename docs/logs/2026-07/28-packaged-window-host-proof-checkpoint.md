# Packaged Window Host Proof Checkpoint

Date: 2026-07-28
State: planning gate resolved; execution paused

## Environment

- macOS 26.5.2, arm64
- Xcode 26.6
- macOS SDK 26.5
- Rust and Cargo 1.96.0 current toolchain
- Rust targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
  `wasm32-unknown-unknown`
- Tauri CLI 2.11.4
- workspace Tauri 2.10.3
- Tauri Runtime 2.10.1
- Tauri Utils 2.9.3
- Bun 1.3.14

The environment can build and operate a packaged macOS arm64 proof. No Windows
or Linux native operator host is available. Those platforms remain
unexecuted, not inferred.

## Dependency Floor

Contract 012 keeps Rust 1.85 until an explicit compatibility change.
`cargo +1.85.0 check -p longhorn-tauri-windowing --all-targets` currently
stops before Longhorn compilation.

The incompatible paths are:

- Tauri Utils through `serde_with` 3.21.0 and Darling 0.23.0
- Tauri and Tauri Utils through plist 1.10.0 and time 0.3.54
- Tauri URL handling through IDNA and ICU 2.2.0

Card 022 starts by resolving the locked graph to the latest sound
Rust-1.85-compatible versions. It then runs the complete workspace check on
Rust 1.85 and current-toolchain QA. A required MSRV increase is a stop
condition and Contract 012 decision, not an implementation fallback.

## Posture

`strict-paused`

Card 022 is fully scoped and the checkpoint evidence is recorded. Execution
still requires an explicit operator start. Bare continuation does not enter
packaged proof.
