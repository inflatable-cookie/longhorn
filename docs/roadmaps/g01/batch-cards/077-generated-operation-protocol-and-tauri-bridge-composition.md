# 077 Generated Operation Protocol And Tauri/Bridge Composition

Status: complete
Owner: Tom
Roadmap: g01.012 batch 2
Governing refs: contracts 001, 007, 010, 012, and 015; research memo 016
Depends on: Card 076
Auto-start next card: no

## Objective

Generate the payload-free operation protocol and prove equal direct, Tauri,
and bridge-domain semantics without making transport metadata catalogue
authority.

## Scope

- Rust-generated checked TypeScript types and fixtures
- operation snapshots, entries, commands, receipts, and errors
- framework-neutral `@longhorn/operation` client
- direct and serialized conformance transports
- `longhorn-tauri-operation` handler/event assembly
- `@longhorn/operation/tauri` composition
- bridge-domain mapping for correlation, progress, cancellation, and terminal
- listener-first snapshot/event reconciliation
- narrow Tauri capability examples

## Out Of Scope

- Svelte or Poodle
- product payloads, commands, reports, logs, or artifacts
- generic bridge command routes
- production network transport
- executor or queue hosting
- notifications

## Steps

1. Define the strict versioned operation wire protocol.
2. Generate exact TypeScript and Rust-produced golden fixtures.
3. Implement the framework-neutral checked client.
4. Prove direct and serialized parity.
5. Add injected Tauri handler assembly and narrow event publication.
6. Add listener-before-snapshot epoch/revision reconciliation.
7. Map optional bridge job metadata onto operation transitions.
8. Prove bridge correlation never replaces operation identity.
9. Audit capabilities, payloads, dependencies, and optional subpaths.

## Acceptance Criteria

- Rust-generated wire fixtures match checked TypeScript exactly
- direct, serialized, Tauri, and bridge traces converge
- gaps and epoch changes force refresh
- stale and duplicate events cannot advance state
- Tauri assembly accepts an injected authority and executor boundary
- bridge job metadata cannot create or mutate catalogue authority alone
- generic protocol carries no product result or progress JSON
- framework-neutral root imports no Tauri, bridge, Svelte, or Poodle

## Evidence Required

- generated fixture diff
- direct/serialized/Tauri/bridge semantic trace matrix
- listener-first race and gap fixtures
- stale session and authority-epoch fixtures
- capability and registered-command audit
- package export and dependency audit
- focused Rust, TypeScript, clippy, docs, formatting, and Effigy checks

## Stop Conditions

- bridge request identity must replace operation identity
- direct and bridge semantics require different public states
- product payloads must enter the generic wire contract
- Tauri adapter must own executor policy
- listener-first reconciliation cannot close the snapshot/event race

## Next Task

Execute ready Card 078. Add per-instance Svelte state and public-Poodle
operation projections.

## Completion Evidence

- Rust serde authority generates the checked TypeScript protocol and golden
  fixture with a zero-diff check.
- Direct, JSON-serialized, Tauri, and bridge-domain traces converge on one
  payload-free operation contract.
- Listener-first reconciliation ignores stale and duplicate events and
  refreshes on gaps and epoch changes.
- `longhorn-tauri-operation` composes injected authority and executor ports;
  executor dispatch failure remains separate from committed cancellation.
- Read, cancel, and manage capability examples remain independently grantable.
- Bridge request and job ids are correlation only. Operation id and authority
  cursor remain catalogue identity.
