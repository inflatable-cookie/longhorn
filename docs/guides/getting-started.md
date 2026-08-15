# Getting Started: Zero To Running

Status: checked private adoption guidance
Updated: 2026-08-15
Governing contracts: [002](../contracts/002-composable-workspace-hosting.md),
[004](../contracts/004-configuration-storage-backup-and-recovery.md), and
[005](../contracts/005-settings-and-system-registration.md)

## What You'll Build

A minimal Svelte + Tauri app that stores its preferences through Longhorn: one
config domain, one settings surface, atomic writes, and a restart-proof file.
This is the smallest proven shape — two Longhorn TypeScript packages (plus the
Poodle renderer packages) and six Rust crates. Work through it once and you
know the pattern every larger composition builds on.

> **Temporary note.** Nothing is published: there is no npm/crates.io install
> of Longhorn or Poodle today. The working TypeScript path is `file:`
> dependencies against sibling checkouts (step 4); the published-version path
> arrives with g02.014, blocked on Poodle v0.2.0. The
> [private 0.1 candidate](../reference/private-0-1-candidate.md) receipt is
> frozen history — it records the 17-tarball TypeScript layout of 2026-08-02;
> the tree produces three tarballs today. The Rust steps are stable.

## 1. Prerequisites

- Rust toolchain (edition 2024; the repo floor is 1.95)
- `bun` or `npm` (the proofs use bun 1.3.x)
- the Tauri CLI (2.x) for dev builds
- the Longhorn Rust artifacts from the
  [private 0.1 candidate](../reference/private-0-1-candidate.md) — source
  archives plus Cargo inventory — and a sibling Longhorn checkout for the
  TypeScript `file:` pins in step 4

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

### Registering the shipped commands

Every `longhorn-tauri-*` crate names its public surface explicitly, and the
commands it ships are registered **by qualified path**:

```rust
.invoke_handler(tauri::generate_handler![
    longhorn_tauri_settings::longhorn_settings_registry,
    longhorn_tauri_notifications::longhorn_notifications_snapshot,
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

## 4. TypeScript: Install The Renderer Packages [temporary]

Nothing is on npm today — do not run a registry install for Longhorn or
Poodle packages. Pin the Longhorn packages by `file:` against a sibling
checkout:

```json
{
  "dependencies": {
    "@inflatable-cookie/longhorn": "file:../longhorn/packages/longhorn",
    "@inflatable-cookie/longhorn-tauri": "file:../longhorn/packages/longhorn-tauri",
    "@tauri-apps/api": "2.10.1",
    "svelte": "5.38.6"
  },
  "overrides": {
    "@inflatable-cookie/longhorn": "file:../longhorn/packages/longhorn"
  }
}
```

```sh
bun install
```

One dependency on `@inflatable-cookie/longhorn` covers every selected system
— `./config`, `./settings`, and the rest are export subpaths, not installable
packages. Add `@inflatable-cookie/longhorn-poodle-svelte` by the same `file:`
discipline when the shape selects Svelte/Poodle bindings. The Poodle packages
(`@inflatable-cookie/poodle-svelte`, `@inflatable-cookie/poodle-core`) are
likewise unpublished until Poodle v0.2.0; pin them from a Poodle checkout the
same way. Adjust relative paths to your layout; commit the lockfile.

The `overrides` block is required, not optional. `longhorn-poodle-svelte` and
`longhorn-tauri` peer-depend on `@inflatable-cookie/longhorn` at exact
`0.1.0`. A top-level `file:` dependency does not satisfy that peer for bun, so
install reaches the registry for `0.1.0` and 404s — for a package that is
already on disk. Point the override at the same path as the dependency.

Once Longhorn publishes — the published-version path arrives with g02.014,
blocked on Poodle v0.2.0 — depend by version and delete the `overrides`
entry. It exists only to keep `file:` installs off the registry.

The 17-tarball `npm install ./artifacts/...` recipe this section used to
carry is frozen with the
[private 0.1 candidate](../reference/private-0-1-candidate.md) receipt: it
matches the 2026-08-02 tree, which produced 17 TypeScript tarballs; the
current tree produces three.

### Development inner loop

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
