# Client Lifecycle And Domain-free Tauri Transport

Date: 2026-07-29
Card: 036
State: complete

## Outcome

- added usable private `@inflatable-cookie/longhorn-core`
- added structural invoke/event transport and asynchronous unlisten contracts
- added one checked snapshot connection
- migrated Surface and transfer connection lifetime
- removed transfer-specific transport aliases
- replaced `TauriTransferTransport` with domain-free `TauriTransport`
- made Tauri 2 a peer
- added TypeScript, package, import, failure, and dependency checks

## Lifecycle Evidence

| Case | Result |
| --- | --- |
| event during initial Surface load | first snapshot accepted, one coalesced refresh loads current authority |
| duplicate invalidation hints | one boolean pending refresh, no unbounded queue |
| direct newer transfer event during load | newer event remains current |
| stale Surface revision or older epoch | ignored |
| stale transfer client epoch | ignored |
| dispose before registration resolves | returned unlisten runs exactly once; no snapshot load |
| registration failure | ready rejects with `registration` evidence |
| load or validation failure | ready rejects, listener closes, `snapshot` evidence retained |
| invalid later event | reporter receives `event`; listener closes |
| asynchronous unlisten failure | dispose rejects with `unlisten` evidence |

The connection owns registration, one refresh pump, current snapshot,
failures, and cleanup. Domains inject validation, event interpretation, and
freshness. No layout service was added.

## Package Boundary

| Package | Runtime dependency boundary |
| --- | --- |
| `@inflatable-cookie/longhorn-core` | none |
| `@inflatable-cookie/longhorn-surfaces` | core |
| `@inflatable-cookie/longhorn-transfer` | core and layout |
| `@inflatable-cookie/longhorn-surface-transfer` | core, Surfaces, and transfer |
| `@inflatable-cookie/longhorn-tauri` | core; `@tauri-apps/api` is a peer |

Core imports no host, browser, Svelte, or Poodle package. Tauri imports no
Longhorn domain. Surface-free packages retain no Surface edge.

## API Delta

- removed `SurfaceClientTransport`, `SurfaceUnlisten`,
  `TransferClientTransport`, `TransferClientEventTransport`, and
  `TransferUnlisten`
- added shared `InvokeTransport`, `EventTransport`, and `Unlisten`
- `TransferClientConnection.dispose()` is asynchronous
- Surface and transfer connections expose retained failure evidence
- renamed the raw adapter to `TauriTransport`

Longhorn is pre-1.0. All repository call sites moved in the same card; no
compatibility aliases or silent fallbacks remain.

## Validation

- affected TypeScript checks passed
- core, Surface, transfer, Surface-transfer, and Tauri tests passed
- all five package dry runs passed
- import-safety and dependency audits passed
- full Effigy QA passed

## Limits

- no new layout command/event endpoint
- no Svelte or Poodle behavior
- no public registry claim
- no consumer migration

## Next

Start Card 037.
