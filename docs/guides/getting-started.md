# Getting Started: Zero To Running

Governing contracts: [002](../knowledge/contracts/002-composable-workspace-hosting.md),
[004](../knowledge/contracts/004-configuration-storage-backup-and-recovery.md), and
[005](../knowledge/contracts/005-settings-and-system-registration.md)

## What You'll Build

A minimal Svelte + Tauri app that stores its preferences through Longhorn: one
config domain, one settings surface, atomic writes, and a restart-proof file.
The minimal composition uses two Longhorn TypeScript packages and six Rust
crates. Add the Svelte/Poodle bindings when your app needs their UI.

The examples below use the coordinated Longhorn 0.6.0 release. TypeScript
packages come from npm, and Rust crates come from the matching Git tag.
See the [release contract](../knowledge/contracts/release.md) and
[changelog](../../CHANGELOG.md) for other versions.

## 1. Prerequisites

- Rust toolchain (edition 2024; the repo floor is 1.97.1)
- `bun` or `npm` (the proofs use bun 1.3.x)
- the Tauri CLI (2.x) for dev builds
- Git access to `inflatable-cookie/longhorn` for the Rust dependencies

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
longhorn-config = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }
longhorn-core = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }
longhorn-settings = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }
longhorn-settings-config = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }
longhorn-tauri-config = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }
longhorn-tauri-settings = { git = "ssh://git@github.com/inflatable-cookie/longhorn.git", tag = "v0.6.0" }
```

Keep the scaffold's Tauri dependency. Run `cargo check` in `src-tauri` to
resolve the tagged crates, then commit `Cargo.lock`. Check the committed
resolution with `cargo check --locked`.

### Registering the shipped commands

Every `longhorn-tauri-*` crate names its public surface explicitly, and the
commands it ships are registered **by qualified path**:

```rust
.invoke_handler(tauri::generate_handler![
    longhorn_tauri_settings::longhorn_settings_registry,
])
```

Do not write wrapper commands that delegate to these. A wrapper is a second
place for the command id to live, and the id is a protocol name the renderer
depends on — one consumer's wrappers drifted from the ids the TypeScript port
expected and cost a live debugging session.

The path form is what matters. `#[tauri::command]` puts its generated
`__cmd__*` helper at the crate root, so the qualified path resolves both the
function and the helper `generate_handler!` needs, whatever the crate's module
layout. Importing the bare function name first and registering that does not,
because the helper is not imported with it.

## 4. TypeScript: Install The Renderer Packages

From the app root:

```sh
bun add --exact @inflatable-cookie/longhorn@0.6.0 @inflatable-cookie/longhorn-tauri@0.6.0 @tauri-apps/api@2.10.1 svelte@5.56.8
```

Commit the lockfile. The Tauri and Svelte/Poodle adapters for this release
both require exactly `@inflatable-cookie/longhorn@0.6.0`. Import systems
from public subpaths such as `@inflatable-cookie/longhorn/config` and
`@inflatable-cookie/longhorn/settings`.

If you select the Svelte/Poodle bindings, add the adapter and the Poodle
packages it uses. Poodle 0.4.16 is a checked peer version.

```sh
bun add --exact @inflatable-cookie/longhorn-poodle-svelte@0.6.0 @inflatable-cookie/poodle-core@0.4.16 @inflatable-cookie/poodle-svelte@0.4.16
```

### Development inner loop

For checkout-based development, set the Longhorn dependencies to
`file:../longhorn/packages/<package-dir>` and override
`@inflatable-cookie/longhorn` to `file:../longhorn/packages/longhorn` so Bun
resolves the adapters' exact peer to the same checkout. Adjust the relative
paths to your layout.

`bun install` for a `file:` dependency materializes `node_modules` as a real
directory of per-file symlinks resolved at install time. Edits to existing
files are live; a file *added* under Longhorn afterwards has no link and is
invisible to the consumer until the next install (Vite's optimized-dep cache
can hold the miss even after that). Do not use raw `bun link` for this — from
the consumer repo, link the Longhorn checkout through Effigy so each package
is one directory symlink and new files appear without reinstall:

```sh
effigy deps link bun ../longhorn --dry-run
effigy deps link bun ../longhorn
effigy deps status bun
```

That leaves `package.json` and the lockfile untouched (`--no-save`).
`bun install` can replace the symlinks; re-run the same `effigy deps link bun`
to restore them. Unlink when the session is done:

```sh
effigy deps unlink bun ../longhorn --dry-run
effigy deps unlink bun ../longhorn
```

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
OS, such as `~/Library/Application Support/my-app/config` on macOS or
`$XDG_CONFIG_HOME/my-app` on Linux.

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
