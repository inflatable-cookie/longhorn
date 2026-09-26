# Greenfield Composition Matrix

Status: promoted
Owner: Tom
Updated: 2026-08-02
Contracts: 002, 004-007, 009-014

## Boundary

Greenfield apps select capability packages directly. There is no umbrella
Longhorn package, generated product shell, copied Poodle primitive, or implied
server. Product pages, schemas, commands, payloads, validation, and policy stay
in the app.

The checked roots live in `examples/greenfield-compositions/`. Each shape has
one renderer manifest, capability declaration, startup root, and Rust manifest.
`common/` is lifecycle proof only. `common-rust/` is storage/config proof only;
neither is a proposed runtime package.

## Compositions

| Shape | Hierarchy | Deliberately absent |
| --- | --- | --- |
| minimal | window | layout, Surface, bridge, history, native content |
| workspace | window, Surface, region, panel | transfer, bridge, history, native content |
| full hosting | display, window, Surface, region, panel | bridge, native content, operations, notifications |
| optional server | window, optional bridge | layout, Surface, history, native content |

Package-count columns were dropped on 2026-08-15: they dated from the
17-package era and drifted silently. The greenfield roots' manifests under
`examples/greenfield-compositions/` are the live selection to copy.

Every renderer also selects the five public Poodle packages reached by the
component and token roots. One Svelte 5.38.6 and Tauri API 2.10.1 runtime
resolves in each isolated install.

Full hosting selects linear history only. Forkable history remains an explicit
later choice. Optional server makes `longhorn-bridge` and
`longhorn-tauri-bridge` feature edges. `--no-default-features` removes both and
leaves the six-crate local config/settings authority valid.

## Storage And Startup

Each Rust root supplies a canonical application id to `platform-native-v1`.
The proof resolves typed config and cache paths, registers a generic config
domain, compiles a missing-file default, publishes one coordinated atomic
mutation, reloads it from disk, and tears down temporary roots.

Apps replace the neutral id and domain with their own stable identity and
schemas. They may choose another contracted storage profile. They must keep
path overrides, migration, backup inventory, and product data adapters
explicit. A composition example grants no legacy cleanup authority.

The shared renderer shell exposes loading, failed, and ready states. Failure is
visible. One mounted listener is released on teardown. Apps inject their own
transport, registered domains, pages, commands, and content after those
lifecycle boundaries exist.

## Capability And Extension Points

`capability.json` records the intended hierarchy, systems, storage profile,
permissions, service posture, and failure posture. It is checked evidence, not
a Tauri capability file to copy unchanged. A real app maps the named Longhorn
handler permissions into its own protected-window capabilities.

Extend downward from the nearest shape:

- minimal: add product config domains, settings pages, and backup adapters
- workspace: add window definitions, layout registry, panels, commands, and
  keymaps; Surface remains absent
- full hosting: add Surface definitions, transfer policy, and typed linear
  history payload/apply policy
- optional server: add consumer operations, opaque credential references, and
  supervision policy; local domain authority remains independent

Operations, notifications, and native-content mechanisms are separate opt-in
systems. Their absence here is intentional, not a package limitation.

## Artifact Proof

Run:

```sh
effigy qa:northstar:g01-greenfield-card125
```

The proof packs 13 Longhorn TypeScript packages and five Poodle packages, then
installs, checks, builds, and mounts all four renderers outside either source
workspace. It inventories and archives 26 Rust crates, reconstructs a private
workspace with the supported lock, runs all four roots offline on Rust 1.85,
and runs optional server again without its server feature.

The exact source, artifact-set, inventory, graph, and audit receipt is
`fixtures/greenfield/card125/composition-matrix-v1.json`. The verifier rejects
selected-source drift, sibling resolution, optional-edge leakage, extra
runtimes, donor vocabulary, copied Poodle implementation, umbrella packages,
and package-manager publication.

Set `KEEP_GREENFIELD_COMPOSITION_PROOF=1` only to inspect the disposable
workspace after a failed run.
