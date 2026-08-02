# Greenfield Composition Matrix

Four neutral Tauri/Svelte roots demonstrate capability selection without an
umbrella package or donor product policy.

| Shape | Hierarchy | Selected systems |
| --- | --- | --- |
| `minimal` | window | config, settings |
| `workspace` | window -> layout container -> region -> panel | minimal plus windowing, layout, commands |
| `full-hosting` | display -> window -> Surface -> layout container -> region -> panel | workspace plus Surface, transfer, linear history |
| `optional-server` | window plus optional service bridge | minimal plus checked bridge and removable supervision |

Each renderer manifest names released-shape versions. Each Rust manifest uses
workspace dependencies only because the private artifact verifier rebuilds a
disposable workspace from source inventories. Neither form resolves the live
Longhorn or Poodle workspace during proof.

Replace each neutral `com.example.longhorn.greenfield.*` id before adopting a
root. Treat `capability.json` as checked selection evidence and map it into the
app's own Tauri capabilities; it is not a drop-in product policy.

The shared mounted shell proves loading, visible failure, ready authority, and
listener teardown. The shared Rust proof resolves a canonical platform-native
storage layout, performs a registered atomic config mutation, reloads the
published value, and drops its temporary roots. Shape roots add only their
selected systems.

Run:

```sh
effigy qa:northstar:g01-greenfield-card125
```

Set `KEEP_GREENFIELD_COMPOSITION_PROOF=1` to retain the disposable artifact
workspace after a failed or exploratory run. No package is published.

The exact package graphs, extension points, and storage/startup boundaries are
recorded in `docs/architecture/greenfield-composition-matrix.md`.

Adoption starts at the checked
[package-selection guide](../../docs/guides/package-selection.md). Storage
identity, profiles, migration, backup, restore, and cleanup are in the
[storage guide](../../docs/guides/storage-configuration-backup.md).
