# Storage, Configuration, Backup, And Recovery

Status: checked private adoption guidance
Updated: 2026-10-22
Governing contracts: [004](../knowledge/contracts/004-configuration-storage-backup-and-recovery.md)
and [005](../knowledge/contracts/005-settings-and-system-registration.md)

## Why This Matters

Where files live and how writes happen is where desktop apps break: config
silently lost on update, backups that cannot be restored, paths that differ
across platforms. Longhorn makes storage a registered contract — stable paths,
atomic writes, recoverable backups — so the app gets durable behavior without
reinventing file handling. This guide covers the parts that touch the
filesystem; the practical register-and-write flow is below under
[Configuration In Code](#configuration-in-code).

## Identity First

Register three different names deliberately:

| Name | Example | Storage effect |
| --- | --- | --- |
| canonical application id | `com.example.product` | default storage leaf and fixed locator identity |
| display name | `Product` | none |
| optional stable storage name | `Product` | replaces the leaf in every selected profile root |

The canonical id is immutable and normally matches the Tauri/bundle id. The
stable storage name is an explicit app-level compatibility choice, not a user
preference. It is never derived from display name. Invalid explicit names fail
closed. Adding, changing, or removing one requires a storage transition.

Default to the canonical id. Choose a stable name only when human filesystem
discoverability and long-lived product-family placement justify the permanent
compatibility promise.

## Default Profile: `platform-native-v1`

`<leaf>` is the canonical id unless the app registers a stable storage name.
The host supplies platform directory facts; pure resolution does not inspect
the environment or filesystem.

| Purpose | macOS | Windows | Linux |
| --- | --- | --- | --- |
| config | `~/Library/Application Support/<leaf>/config` | `%LOCALAPPDATA%\<leaf>\config` | `$XDG_CONFIG_HOME/<leaf>` |
| data | `~/Library/Application Support/<leaf>/data` | `%LOCALAPPDATA%\<leaf>\data` | `$XDG_DATA_HOME/<leaf>` |
| state | `~/Library/Application Support/<leaf>/state` | `%LOCALAPPDATA%\<leaf>\state` | `$XDG_STATE_HOME/<leaf>` |
| cache | `~/Library/Caches/<leaf>` | `%LOCALAPPDATA%\<leaf>\cache` | `$XDG_CACHE_HOME/<leaf>` |
| logs | `~/Library/Logs/<leaf>` | `%LOCALAPPDATA%\<leaf>\logs` | `$XDG_STATE_HOME/<leaf>/logs` |
| runtime | system temporary directory plus `<leaf>` | system temporary directory plus `<leaf>` | `$XDG_RUNTIME_DIR/<leaf>` |
| backups | `~/Library/Application Support/<leaf>/backups` | `%LOCALAPPDATA%\<leaf>\backups` | `$XDG_DATA_HOME/<leaf>/backups` |

Packaged Windows may supply package-isolated local/cache facts. That is not a
roaming or synchronization claim. Linux paths use injected XDG facts; the app
must diagnose unavailable required facts rather than inventing a home-dotfile
fallback. Longhorn never silently writes to the current directory or
`~/.product`.

## Other Profiles

| Profile | Shape | Use | Cost |
| --- | --- | --- | --- |
| `unified-app-root-v1` | typed children under one native durable app root | one app-owned tree matters more than native lifecycle separation | cache/runtime/backup classification warnings |
| `shared-product-root-v1` | typed children under one injected shared product-data root | cooperating product processes need one filesystem root | no multi-writer or sync semantics; Windows uses roaming `%APPDATA%` |
| `portable-v1` | typed children under one explicit absolute root | intentional portable/test deployment | root must be supplied; never guesses executable/current directory |

`shared-product-root-v1` uses `Application Support`, `%APPDATA%`, or
`$XDG_DATA_HOME` as parent. It preserves leaf case. Per-purpose overrides are
deployment/test policy and appear in the layout receipt; they are not a
substitute for choosing a coherent profile.

Profile ids are versioned path contracts. An existing id never changes its
resolution when Longhorn is upgraded.

## Longhorn For CLI Apps

A plain Rust program — a CLI with no Tauri stack — can adopt the same
storage policy without the config store, backup, or webview machinery.
Everything in this section is the `longhorn-storage-layout` crate: storage
identity, the pure profile resolver, `StorageRoots`, and platform-directory
facts. `longhorn-config` builds its store, backup, and transition machinery
on that crate and does not re-export it, so a CLI never pulls the store to
resolve its paths.

The identity rules are the ones at the top of this guide: one immutable
canonical application id, optionally one explicit stable storage name that
replaces the leaf. The id is the default leaf, so an unconfigured CLI lands
in the same directories a Tauri application with the same id lands in.

A CLI supplies platform-directory facts natively instead of through Tauri:

```rust
use longhorn_storage_layout::{
    RootKind, StorageIdentity, StorageLayoutRequest, native_platform_directory_facts,
    resolve_storage_layout,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let identity = StorageIdentity::new("com.example.product")?;
    let facts = native_platform_directory_facts()?;
    let layout = resolve_storage_layout(&StorageLayoutRequest::new(identity, facts))?;
    println!(
        "config: {}",
        layout.root(RootKind::Config).unwrap().path().display()
    );
    Ok(())
}
```

The native provider reads the environment once and maps it exactly the way
the Tauri adapter does, so the paths match on every platform:

| Platform | Facts source |
| --- | --- |
| macOS | `$HOME/Library/Application Support`, `$HOME/Library/Caches`, `$HOME/Library/Logs`, system temp |
| Linux | `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` with the spec defaults `$HOME/.config`, `$HOME/.local/share`, `$HOME/.local/state`, `$HOME/.cache`; `XDG_RUNTIME_DIR` has no default |
| Windows | `LOCALAPPDATA`, roaming `APPDATA` for shared data, system temp |

A missing `HOME`, an unset `XDG_RUNTIME_DIR`, or a missing Windows
known-folder variable leaves its facts unavailable; resolution reports the
gap instead of inventing a home dotfile. Per-purpose roots and profiles are
overridable the same way as for a Tauri app — the env snapshot form
(`native_platform_directory_facts_from`) is pure and testable, and the
resolver rejects relative roots and paths.

The full store (validated domains, atomic mutation, backup, restore) stays
in `longhorn-config`; a CLI that needs it composes the same
[`StorageRoots`](#configuration-in-code) round-trip shown below. TOML as a
human-writable config format for CLI domains arrives in a later `0.4.0`
step of the `cli-adoption` lane; JSON domains are the format until then.

## Domains And Databases

Register one stable domain id, schema version, codec, default, storage class,
and confined relative path per authority/mutation cadence. Keep preferences,
window state, layout, caches, and credentials separate. Cross-domain atomicity
requires an explicit transaction authority.

Storage classes choose meaning before mechanism:

- user intent → config
- display/window/device correlation → machine state
- workspace-personal state → state keyed by workspace
- intentionally shared project state → explicit project/server authority
- recreatable indexes → cache
- credentials → secure-store adapter
- ephemeral logs/runtime → their lifecycle roots

A database does not define its class. Durable app-owned databases normally
live below `data/databases`; machine restart state uses state; rebuildable
indexes use cache. Live SQLite uses native backup/restore APIs. Never copy a
live main/WAL pair as ordinary files.

## Read, Mutation, And Flush

Missing domain files return compiled defaults with a diagnostic. Corrupt or
future files remain intact and enter typed recovery. Older schemas migrate
through ordered idempotent steps after a verified pre-migration backup.

Ordinary writes use one stable store-wide coordination authority:

1. take the process mutex and finite-deadline OS advisory lock
2. reread current authority
3. apply a typed patch, validate, and encode
4. create and sync a unique sibling temporary file
5. atomically replace the target and sync the directory where supported
6. report achieved durability

Failed pre-publication writes leave the last valid file intact. Advisory locks
coordinate participating local writers only; they do not exclude direct
external writers or provide multi-machine transactions.

Debounce is opt-in for bounded typed intent. It is not a desired-document
cache. Close/shutdown force-flushes every lane and handles each receipt. A
pre-publication failure retains intent for explicit retry or discard. A known
replacement with durability failure clears intent to prevent duplicate apply.

## Configuration In Code

A complete register, load, and mutate round-trip. This is the exact shape
proved by the greenfield examples (see
`examples/greenfield-compositions/common-rust/src/lib.rs`).

```rust
use std::time::Duration;

use longhorn_config::{
    ConfigDomain, ConfigStore, CoordinationAuthority, DomainDescriptor, DomainFilePath,
    DomainIssue, DurabilityRequirement, LoadedOrigin, MigrationStep, MutationOptions,
    StorageClass,
};
use longhorn_core::{DomainId, SchemaVersion};
use longhorn_storage_layout::StorageRoots;
use serde_json::{json, Value};

// A domain is a typed, validated slice of config owned by one authority.
struct Preferences {
    descriptor: DomainDescriptor,
}

impl Preferences {
    fn new() -> Self {
        Self {
            descriptor: DomainDescriptor::new(
                DomainId::new("com.example.product.preferences").unwrap(),
                SchemaVersion::new(1).unwrap(),
                StorageClass::UserConfig,
                Some(DomainFilePath::new("preferences.json").unwrap()),
            )
            .unwrap(),
        }
    }
}

impl ConfigDomain for Preferences {
    type Value = Value;

    fn descriptor(&self) -> &DomainDescriptor {
        &self.descriptor
    }
    fn default_value(&self) -> Self::Value {
        json!({ "enabled": false })
    }
    fn decode(&self, value: Value) -> Result<Self::Value, DomainIssue> {
        self.validate(&value)?;
        Ok(value)
    }
    fn encode(&self, value: &Self::Value) -> Result<Value, DomainIssue> {
        Ok(value.clone())
    }
    fn validate(&self, value: &Self::Value) -> Result<(), DomainIssue> {
        if value.get("enabled").is_some_and(Value::is_boolean) {
            Ok(())
        } else {
            Err(DomainIssue::new("enabled", "enabled must be boolean"))
        }
    }
    fn validate_raw(&self, version: SchemaVersion, value: &Value) -> Result<(), DomainIssue> {
        if version == self.descriptor.schema_version() {
            self.validate(value)
        } else {
            Err(DomainIssue::new("version", "unsupported schema version"))
        }
    }
    fn migrate_one(
        &self,
        _from: SchemaVersion,
        _value: Value,
    ) -> Result<Option<MigrationStep>, DomainIssue> {
        Ok(None)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Resolved roots for the app's platform and profile (see the profile
    // tables above); this example uses explicit portable-style roots.
    let base = std::path::PathBuf::from("data");
    let roots = StorageRoots::new(
        &base.join("config"),
        &base.join("data"),
        &base.join("state"),
        &base.join("cache"),
        &base.join("runtime"),
        &base.join("logs"),
        &base.join("backups"),
    )?;
    let mut store = ConfigStore::new(roots, CoordinationAuthority::new(&base.join("data"))?);

    let preferences = Preferences::new();
    store.register(&preferences)?;

    // First load returns the compiled default and reports where it came from.
    let first = store.load(&preferences)?;
    assert_eq!(first.origin, LoadedOrigin::Default);

    // A mutation is a typed patch applied atomically under the store lock.
    let receipt = store.mutate(
        &preferences,
        MutationOptions::new(Duration::from_secs(1), DurabilityRequirement::Atomic),
        |value| {
            value["enabled"] = Value::Bool(true);
            Ok(())
        },
    )?;

    // The next load reads the published file.
    let reloaded = store.load(&preferences)?;
    assert_eq!(reloaded.origin, LoadedOrigin::File);
    Ok(())
}
```

## Human TOML Config For CLI Apps

CLI and Tauri apps share one config system. An app that wants a
hand-editable file declares a domain whose file is human-written TOML. This
is opt-in per domain behind the `toml` feature of `longhorn-config`; without
the feature no TOML code compiles and JSON domains are byte-identical.

Declare the descriptor with `DomainDescriptor::new_toml` and a `.toml` file
path. The file lives at the domain's resolved location: the platform config
root (see the profile tables above) plus the registered relative path, for
example `~/Library/Application Support/<leaf>/config/cli-settings.toml` on
macOS or `$XDG_CONFIG_HOME/<leaf>/cli-settings.toml` on Linux.

The file holds the domain value only — no envelope:

```toml
# cli-settings.toml — schema_version is optional and reserved.
# Without it, the file is read at the domain's current schema version.
schema_version = 2

name = "cli"
enabled = true

[profile]
tags = ["fast", "quiet"]
```

Type mapping follows contract 004: tables become objects, arrays become
arrays, integers stay 64-bit signed, floats must be finite, and offset
date-times (`1979-05-27T07:32:00Z`) become RFC 3339 strings. TOML has no
null. `nan`, `inf`, integers beyond 64 bits, and local datetimes without a
UTC offset are refused with the reason stated.

The file is read-only in this phase. Longhorn reads, validates, and migrates
it in memory and never writes it — every mutation, debounced flush, migration
rewrite, and restore into the domain refuses with a typed error naming the
domain as human-written and read-only. Migration leaves the file exactly as
the person wrote it. Backups record the domain as an explicit exclusion
(`human-written, read-only in phase 1`) instead of capturing it.

A broken file never touches the last good state. Errors come back as a typed
recovery state with a one-based line and column: parse, `schema_version`, and
unmappable-value refusals point at the exact span, and domain validation
issues point at the document start. A half-saved editor buffer is just a
parse error — the next load reads the file again.

```rust
let descriptor = DomainDescriptor::new_toml(
    DomainId::new("com.example.product.cli-settings").unwrap(),
    SchemaVersion::new(2).unwrap(),
    StorageClass::UserConfig,
    Some(DomainFilePath::new_toml("cli-settings.toml").unwrap()),
)?;
```

## Profile Selection And Migration

A fixed minimal locator lives at the canonical-id `platform-native-v1` path.
It selects the profile and last committed layout without depending on the root
it selects. Corrupt, future, or unknown locators enter recovery; they never
silently fall back.

Profile or stable-name change is a transaction:

1. preview source and target layouts
2. inventory ordinary and custom stores
3. reject overlap, ambiguity, and destination conflicts
4. acquire both authorities in stable order
5. stage and verify files and native database snapshots
6. journal beside the fixed locator
7. commit the locator last
8. retain old storage until exact receipt-bound cleanup is authorized

Cache is excluded by default, runtime is never migrated, logs are optional
evidence, and secrets remain in secure storage. Legacy discovery is read-only.
It does not merge, delete, or adopt dot directories or old Tauri roots.

## Backup

Backup is registry capture, not a directory copy. Every domain is included,
excluded with reason, or custom-adapted. Secrets, cache, runtime, and logs are
excluded by default. Capture includes published state only; force-flush first
when pending intent matters.

Ordinary domains captured under the Longhorn lock share one consistency group.
External snapshots such as SQLite normally form another unless the consumer
supplies a higher transaction authority.

The portable inner bundle is deterministic ZIP:

- media type `application/vnd.longhorn.config-backup+zip`
- extension `.longhorn-backup`
- strict manifest first at `longhorn/manifest.json`
- ordinary payloads under `longhorn/domains/`
- adapter payloads under `longhorn/adapters/`
- bounded entries, bytes, ratios, regular files, and confined paths

Optional encryption wraps the entire ZIP in binary age v1 with extension
`.longhorn-backup.age`. Keys and passphrases never enter ordinary config.
Unavailable identities mean locked, not corrupt. Plain SHA-256 proves byte
integrity, not authenticity.

Operational backups use the selected backup root and safe retention. User
exports use an explicit user-selected parent and canonical re-encoding.
Retention never auto-deletes the new, pinned, locked, corrupt, unreadable,
foreign, unknown-version, or unparseable archive.

## Restore And Recovery

Inspection does not mutate. It validates bounds, app identity, payload hashes,
schemas, migrations, adapters, conflicts, and current evidence. The confirmed
plan binds the archive digest, choices, and current target state. Execution
rechecks freshness.

Restore promises failure-atomic terminal state across independent files, not
instantaneous cross-file visibility:

- success: every selected domain matches staged target evidence
- rolled back: every selected domain matches captured source evidence
- recovery required: rollback is unverified; normal writes remain blocked

Execution stages and validates the complete set, captures rollback evidence,
publishes a safety backup and durable journal, replaces through the ordinary
atomic publisher, verifies, and either commits or restores. Crash recovery
finishes verified rollback before later mutation.

Custom adapters have two explicit restore modes. `Separate` runs one domain as
a separately confirmed and receipted operation. `GroupedFailureAtomic` joins a
selected set through the grouped protocol:

1. plan one exact sorted domain set from one inspected archive
2. confirm one digest covering the archive, adapters, target, and rollback evidence
3. quiesce all consumer-owned authorities
4. stage every opaque target and exact rollback payload without mutation
5. persist every payload and one journal before the first publication
6. apply and verify all targets, or unwind all domains and verify old evidence
7. on interruption, recover with the exact catalogue before opening authorities

Custom semantic state is explicit: `Absent`, or `Present` with a SHA-256
digest. Absent archive targets and absent rollback states each use zero
payloads, apply as deletion, and verify absence. Present states require one or
more opaque payloads. Never substitute an empty document, sentinel payload, or
synthetic digest.

The grouped boot path is Rust-only and renderer-free. Longhorn owns the
transaction, journal, rollback, and receipts. The app still owns shutdown,
SQLite/WAL policy, domain meaning, restart scheduling, and presentation. See
the [public API reference](../reference/grouped-adapter-restore.md).

## Settings Presentation

Settings projects registered storage and domain authority. It sends validated
patches or explicit operations; it never edits files. Show profile, identity,
resolved roots, provenance, warnings, backup consistency groups, restore
choices, and exact terminal receipts. Preserve `recoveryRequired` as a blocking
state rather than presenting a generic failure or silent reset.
