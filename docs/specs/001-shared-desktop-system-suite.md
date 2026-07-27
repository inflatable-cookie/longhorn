# 001 Shared Desktop System Suite

Status: active  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/001-tauri-application-extraction-audit.md`,
`../research/translation-memos/002-shared-desktop-systems-follow-up.md`

## Purpose

Keep the complete candidate suite and its maturity in one durable place.
Architecture and contracts own promoted behavior. This spec owns unresolved
shape and admission questions.

## Suite

| Capability | Intended reusable unit | State | Main evidence |
| --- | --- | --- | --- |
| configuration storage | Rust domain stores, roots, migrations, safe writes | promoted first pass | Loophole, Soundcheck, Bovine, Nucleus |
| backup and recovery | inventory, snapshot, verify, rotate, restore | promoted first pass | cross-app need; partial Loophole evidence |
| display inventory | pure model plus Tauri probe adapter | contract 009 | Loophole, Nucleus, Soundcheck |
| geometry/window planning | pure planner plus native host | contract 009 | Loophole, Nucleus, Soundcheck |
| lifecycle and app chrome | small Tauri/Svelte helpers and guidance | provisional | all five apps |
| layout containers | Rust/TS region and panel state | promoted first pass | Loophole, Nucleus |
| hosted Surfaces | optional layout-container host | promoted first pass | Loophole |
| cross-window drag | id/coordinate protocol and host transaction | contract 011 | Loophole, Nucleus |
| Svelte/Poodle bindings | stores, actions, adapters; no component fork | architecture | Loophole, Nucleus |
| settings | registry-driven dialog shell and domain pages | promoted first pass | Loophole plus cross-app demand |
| typed bridge | checked commands, events, errors, subscriptions | contract 010 | all five apps |
| optional backend topology | local, embedded, or remote authority adapters | promoted first pass | Nucleus, Loophole |
| commands/actions | registry, context, availability, execution | promoted first pass | Loophole, Jetstream |
| input/keymaps | triggers, contexts, conflicts, overrides | promoted first pass | Loophole |
| command palette | Poodle-backed projection of command registry | promoted first pass | Loophole |
| history | generic linear kernel, then branch prototype | promoted research boundary | Loophole |
| async operations | progress, cancellation, stale-request protection | incubating | Soundcheck |
| notifications | bounded host events and UI projection | incubating | Loophole, Soundcheck |
| native content islands | geometry/input host adapters | prototype required | Nucleus, Soundcheck, Jetstream |
| greenfield starter | composition examples and release guidance | runway | all consumers |

## Cross-System Shape

- Rust owns durable state, validation, mutations, and host authority.
- TypeScript owns checked projections and framework-neutral interaction.
- Svelte owns subscriptions and presentation composition.
- Poodle owns visual primitives.
- Apps register product definitions and policy.
- Optional systems depend downward; the foundation never depends on Surface,
  Poodle, a specific server, or a product domain.

## Configuration Questions

- exact domain registration API
- file-per-domain versus grouped transaction boundaries
- multi-process locking support in v1
- backup archive format and encryption
- policy/config precedence for managed installations
- server-synchronized configuration conflict semantics

## History Questions

- whether the generic payload is typed by a trait, opaque serialized data, or
  a consumer-owned enum
- how apply and inverse failure leave state and history
- whether coalescing is payload behavior or stack policy
- how persisted entries migrate with product schemas
- branch identity, pruning, annotations, checkpoints, and recovery
- which histories are local UI state versus shared product state

## Promotion Rules

- No roadmap may treat an open question as implied behavior.
- A generic system needs two materially different fixtures or one strong
  greenfield invariant before implementation.
- A Poodle component remains in Poodle. Longhorn may supply its state and
  registration model.
- An optional system stays a separate dependency and feature boundary.
- A donor implementation is evidence, not the public API.

## Rejected Shapes

- one all-app framework
- one `settings.json` containing unrelated domains
- configuration files in the current working directory by default
- secrets mixed into ordinary config or backups
- renderer-authoritative durable state
- Surface as a mandatory parent of regions
- palette definitions separate from the command registry
- claiming branch history from a linear undo/redo donor
