# g01.001 Foundation Contracts And Package Topology

Status: complete  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: architecture inventory; package topology; contracts 001-013

## Outcome

Freeze the minimum public boundaries and workspace/package graph needed for
implementation without turning Longhorn into one framework.

## Batches

### 1. Contract closure

- close display identity and coordinate contracts
- close window-planning and Tauri host contracts
- close Rust/TypeScript IPC/event and drag contracts
- review contracts 004-008 against two-consumer fixtures

### 2. Package graph

- choose Rust crates and dependency direction
- choose TypeScript core, Svelte, and Poodle adapter packages
- keep Surface, server, history, native-island, and Poodle dependencies optional
- name public versus internal packages

### 3. Distribution and test topology

- choose workspace, versioning, publication, and local-consumer strategy
- define cross-language fixture ownership
- replace docs-only test discovery with Rust/TS discovery
- define packaged-Tauri validation lanes

## Acceptance

- no active foundation roadmap depends on a pending boundary
- dependency graph has no upward optional-module edge
- Loophole full-hosting and Nucleus no-Surface compositions fit
- Bovine minimal composition imports only selected packages
- `effigy test --plan` discovers the intended package checks

## Gate

No implementation batch card before contract and package-graph review.

## Closeout

- promoted display/window, IPC/event, transfer, distribution, and
  Svelte/Poodle lifecycle contracts
- froze the optional, downward-only Rust and TypeScript package graph
- preserved donor evidence in translation memo 003
- compiled the first ready `g01.002` card

## Next Task

Execute `batch-cards/001-configuration-domain-store.md`.
