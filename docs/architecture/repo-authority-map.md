# Repo Authority Map

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Architecture: `system-architecture.md`, `system-inventory.md`

## Topology

Longhorn becomes the mechanism authority. Consumer repos retain product
authority and migrate through explicit batches. Poodle remains independent.

## Repo Authorities

| Repo | Owns | Consumes from Longhorn | Must not move |
| --- | --- | --- | --- |
| `longhorn` | generic Rust/TS/Svelte desktop mechanisms | n/a | app schemas, commands, history payloads, panel catalogues, workflows |
| `poodle` | components, tokens, interaction primitives | optional shared types only | host state and persistence |
| `loophole` | DAW shell policy, mutations, and panel catalogue | full hosting, config, command, settings, optional history stack | Pulse/Signal/Aura product authority |
| `nucleus` | agent workspace policy, resources, and server data | window + region/panel, config, settings, topology adapters | project/task/runtime authority |
| `soundcheck` | plugin-library workflows | foundation modules as adopted | scan/sync/inspection domain |
| `jetstream` | engine/editor and native renderer | bridge or native-host adapters as adopted | frame/render/world authority |
| `acowtancy/bovine-accelerator-desktop` | content workspace | simple preferences/layout adapters as adopted | content/navigation/editorial domain |

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
- Apps register configuration schemas, settings pages, commands, and history
  payloads; registration does not transfer product authority.
- A service cache never becomes a second write authority by accident.
- Audit access is read-only. Consumer writes need an explicit migration card.

## Planning Gaps

- package publication/path-dependency strategy
- first two consumer migration order
- ownership transfer plan for generic Echo crates
