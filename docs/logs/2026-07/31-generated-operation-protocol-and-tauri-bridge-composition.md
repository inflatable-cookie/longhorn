# Generated Operation Protocol And Tauri/Bridge Composition

Date: 2026-07-31
Card: 077
Status: complete

## Changed

- Added strict payload-free operation snapshots, commands, receipts,
  rejections, cancellation dispatch evidence, and changed events in Rust.
- Added Rust-generated checked TypeScript and golden protocol fixtures.
- Added `@inflatable-cookie/longhorn-operation` with strict compatibility checks, listener-first
  reconciliation, and direct plus JSON-serialized adapters.
- Added optional Tauri and bridge subpaths. The root imports neither.
- Added `longhorn-tauri-operation` over separate injected authority and
  executor ports with read, cancel, and manage capability examples.
- Added cumulative terminal-eviction evidence to catalogue snapshots.

## Decisions

- Operation request ids are distinct from operation ids and bridge request or
  job ids.
- Events are non-durable invalidation hints. Snapshots remain authority.
- Executor dispatch happens only after cancellation authority commits.
  Dispatch failure stays visible without changing the committed result.
- Bridge metadata may correlate progress or terminal hints only when bridge
  and operation identities both match.
- Product result, artifact, report, and log payloads remain forbidden in the
  generic operation protocol.

## Evidence

- Semantic trace matrix:

  | Transport | Snapshot | Mutation | Cancellation | Authority |
  | --- | --- | --- | --- | --- |
  | direct | exact v1 | exact receipt | exact receipt + dispatch | catalogue |
  | serialized | JSON clone | same | same | catalogue |
  | Tauri | typed invoke | same + hint | same + executor + hint | injected host |
  | bridge | query envelope | command envelope | cancellation envelope | catalogue; bridge correlation only |

- Rust fixture generation and zero-diff binding check pass.
- 21 pure operation contract tests pass.
- 4 Tauri operation host and capability tests pass.
- 11 TypeScript protocol, transport, bridge, Tauri, reconciliation, and
  package-boundary tests pass.
- Focused Rust clippy passes with all targets and binding features.
- Dry-run package assembly contains only the intended root and optional
  adapter subpaths.
- Full repository `effigy qa` passes.

## Next

Execute Card 078. Add per-instance Svelte operation sessions and controlled
public-Poodle projections for Soundcheck scan and Loophole render shapes.
