# Repo Authority Map

Status: active first pass  
Owner: Tom  
Updated: 2026-08-02
Architecture: `system-architecture.md`, `system-inventory.md`,
`secondary-consumer-migration-map.md`

## Topology

Longhorn becomes the mechanism authority. Consumer repos retain product
authority and migrate through explicit batches. Poodle remains independent.

## Repo Authorities

| Repo | Owns | Consumes from Longhorn | Must not move |
| --- | --- | --- | --- |
| `longhorn` | generic Rust/TS/Svelte desktop mechanisms | n/a | app schemas, commands, history payloads, panel catalogues, workflows |
| `poodle` | components, tokens, interaction primitives | optional shared types only | host state and persistence |
| `loophole` | DAW shell policy, history payload/apply/recovery, and panel catalogue | full hosting, config, command, settings, and structural linear history | Pulse/Signal/Aura product authority |
| `nucleus` | agent workspace policy, resources, server data, panel and Browser policy | storage, protected window, registered layout, checked renderer, native content | project/task/runtime authority |
| `soundcheck` | plugin-library workflows and desktop policy | stable-name storage, config, window, settings/recovery, operation, isolated-window coordination | soundcheck-library SQLite/scan plus Signal plugin, DAW, sync, and inspection policy |
| `jetstream` | engine/editor and native renderer | bridge, command/keymap, backing-surface coordination | command execution, frame/render/WGPU/world/input authority |
| `acowtancy/bovine-accelerator-desktop` | content workspace | minimal config/settings and public Poodle artifacts | content/navigation/editorial/Git domain; no forced layout |

## Cross-Repo Rules

- Donor code remains authoritative until its migration batch passes.
- After cutover, Longhorn owns shared behavior and the donor copy is removed.
- Consumer policy enters Longhorn only through generic configuration or traits.
- Rust/TypeScript wire changes require Longhorn contract and fixture updates.
- Poodle integration uses public Poodle APIs; no copied component fork.
- Longhorn may request public drag extension points through a named Poodle
  upstream card. Poodle retains API, interaction, and release authority.
- Private Poodle selectors, generated ids, MIME formats, and source aliases do
  not become Longhorn contracts.
- Apps register configuration schemas, settings pages, commands, history
  payload policy, and atomic apply transactions; registration does not
  transfer product authority.
- A service cache never becomes a second write authority by accident.
- Audit access is read-only. Consumer writes need an explicit migration card.

## Planning Gaps

- later release-lane registry ownership, public names, and publication policy

The generic Echo transfer is complete under `loophole-migration-map.md`.
Product schemas, policies, payloads, and named adapters remain in Loophole.
Secondary-consumer selection and retained authority are compiled under
`secondary-consumer-migration-map.md`; Card 113 now freezes exact receipts,
behavior seams, selected packages, rollback inputs, and the protected Bovine
docs-only overlap before any new consumer write. Card 119 closes Soundcheck's
selected graph and retains every SQLite, scan, plugin, DAW, Composer,
Keepsake, Signal, Swallowtail, and visual-product authority downstream. Card
120 moves only Bovine's storage, preference persistence, settings structure,
and Tauri transport to Longhorn; content, hierarchy, navigation meaning,
editorial, validation, Git, and SplitView presentation remain downstream.
Card 121 closes the minimal graph with one preference authority, exact
settings-session teardown, retained legacy rollback, and no layout or optional
system edge.
