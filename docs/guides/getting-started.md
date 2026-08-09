# Getting Started: Zero To Running

Status: checked private adoption guidance
Updated: 2026-08-08
Governing contracts: [002](../contracts/002-composable-workspace-hosting.md),
[004](../contracts/004-configuration-storage-backup-and-recovery.md), and
[005](../contracts/005-settings-and-system-registration.md)

## What You'll Build

A minimal Svelte + Tauri app that stores its preferences through Longhorn: one
config domain, one settings surface, atomic writes, and a restart-proof file.
This is the smallest proven shape — four TypeScript packages and six Rust
crates. Work through it once and you know the pattern every larger
composition builds on.

> **Temporary note.** The TypeScript install path is in flux: Poodle's
> artifact layout changes with its upcoming release, and the Longhorn
> renderer-package story is being reworked. Steps marked **[temporary]**
> are current today but will change; the Rust steps are stable. The
> [private 0.1 candidate](../reference/private-0-1-candidate.md) receipt is
> the source of truth for artifact names.

## 1. Prerequisites

- Rust toolchain (edition 2024; the repo floor is 1.95)
- `bun` or `npm` (the proofs use bun 1.3.x)
- the Tauri CLI (2.x) for dev builds
- the Longhorn artifact set from the
  [private 0.1 candidate](../reference/private-0-1-candidate.md) — Rust
  source archives plus Cargo inventory, and the npm-compatible tarballs

## 2. Scaffold The App

```sh
bun create tauri-app
```

Choose a TypeScript/Svelte template and name the app (for example `my-app`).
This creates the `src-tauri` Rust host and the Svelte renderer.

## 3. Rust: Add The Crate Set

Edit `src-tauri/Cargo.toml`:

```toml
[dependencies]
longhorn-config = "0.1.0"
longhorn-core = "0.1.0"
longhorn-settings = "0.1.0"
longhorn-settings-config = "0.1.0"
longhorn-tauri-config = "0.1.0"
longhorn-tauri-settings = "0.1.0"
```

These crates are not on crates.io — do not run `cargo add`. Resolve them
against the candidate's Cargo inventory and commit-derived source archives
with the exact lock from the receipt, then verify offline:

```sh
cargo check --locked
```

## 4. TypeScript: Install The Renderer Packages [temporary]

Install the minimal shape's packages from the produced tarballs, never from a
registry:

```sh
npm install \
  ./artifacts/longhorn-config-0.1.0.tgz \
  ./artifacts/longhorn-core-0.1.0.tgz \
  ./artifacts/longhorn-settings-0.1.0.tgz \
  ./artifacts/longhorn-tauri-0.1.0.tgz \
  ./artifacts/poodle-svelte-0.1.0.tgz \
  ./artifacts/poodle-svelte-tokens-0.1.0.tgz
npm install svelte@5.38.6 @tauri-apps/api@2.10.1
```

Tarball filenames come from the candidate receipt (`artifacts.longhornTypescript`
and `artifacts.poodle`). Commit the lockfile. The exact Poodle tarball names
and this recipe change with the upcoming Poodle release — re-read the
[distribution reference](../reference/private-0-1-candidate.md) when it lands.

## 5. Register The Config Domain

A domain is a typed, validated slice of config. The complete register →
load → mutate round-trip lives in
[Storage, Configuration, Backup, And Recovery](storage-configuration-backup.md#configuration-in-code);
the short version:

```rust
// Register once at startup, before the first window reveals.
store.register(&preferences)?;

// First load returns the compiled default; later loads read the file.
let loaded = store.load(&preferences)?;

// A mutation is a typed patch applied atomically under the store lock.
store.mutate(
    &preferences,
    MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
    |value| {
        value["enabled"] = Value::Bool(true);
        Ok(())
    },
)?;
```

The default storage profile writes to the platform-native location for your
OS — `~/Library/Application Support/my-app/config` on macOS, the equivalent
under `%LOCALAPPDATA%` on Windows, and XDG roots on Linux.

## 6. Run And Verify

```sh
bun run tauri dev
```

What "it works" looks like:

1. First launch: no config file exists; the app runs on the compiled default
   and reports `LoadedOrigin::Default`.
2. Toggle the setting: the file appears at the config path, written
   atomically (temp file, fsync, rename).
3. Quit and relaunch: the app loads from the file
   (`LoadedOrigin::File`); the value survives.

## 7. What's Next

- More systems, one at a time: [Choose Packages](package-selection.md)
- Composition order and boundaries: [Compose Desktop Systems](system-composition.md)
- The same, for a native host: [Compose A GPUI Application](gpui-composition.md)
- Migrating an existing app: [Migrate And Roll Back](migration-and-rollback.md)
- What is claimed, and upgrades: [Compatibility And Upgrades](compatibility-and-upgrades.md)
- Terms: [Glossary](glossary.md)
