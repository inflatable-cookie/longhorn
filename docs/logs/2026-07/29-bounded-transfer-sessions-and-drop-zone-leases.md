# Bounded Transfer Sessions And Drop-zone Leases

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 031
- added the pure `longhorn-transfer` crate
- represented session ids as exactly 128 allocator-supplied bits
- serialized renderer payloads as protocol version plus session id only
- required injected monotonic time and entropy
- bounded sessions, current client windows, leases, zones, insertion
  positions, and lifetimes independently
- admitted sessions only against current client-window epochs
- made cancellation idempotent and terminal target attempts single-use
- invalidated source sessions on client-epoch change and window destroy
- atomically replaced complete per-window leases
- retained the prior generation on every invalid replacement
- resolved explicit zone ids and screen points against fresh window bounds
- rejected overlapping eligible windows and zones without enumeration order
- used one core for direct layout-region and opaque hosted-window targets
- imported no Surface package
- made Card 032 ready

## Session Authority

`DragSessionId` contains `[u8; 16]` and serializes as 32 lowercase hexadecimal
bytes. The allocator trait returns entropy rather than a caller-formatted
string. Longhorn cannot silently substitute an ambient random source.

Session creation order is:

1. observe injected monotonic time
2. validate lifetime and checked expiry
3. confirm current source client and epoch
4. reclaim expired capacity when needed
5. reject full capacity before invoking the allocator
6. allocate exact session entropy
7. reject a current id collision
8. publish one process-local record and minimal payload

Source records contain ids, subject kind, client epoch, and projected revision
only. They contain no panel, Surface, layout, product, or resource snapshot.

## Session State Matrix

| State or event | Result |
| --- | --- |
| active cancellation | cancelled |
| repeated cancellation | idempotent already-cancelled receipt |
| first active terminal attempt | consumed before target resolution |
| terminal replay | typed replay rejection |
| expiry boundary `now == expires_at` | expired |
| source window destroy | active source session invalidated |
| source client epoch advances | active old-epoch session invalidated |
| clock moves backwards | typed regression rejection |
| host discard | sessions, clients, and leases cleared with counts |

Expired entries may be reclaimed when capacity is needed. No transfer state is
persisted.

## Lease Publication

One current client epoch owns each window lease. A newer generation replaces
the complete prior zone set. A new client epoch removes the prior lease and may
restart generation numbering.

Publication rejects before replacement:

- unknown or stale client epoch
- zero, excessive, or overflowing lifetime
- exhausted independent lease capacity
- excessive zone count
- duplicate zone ids inside the lease
- zero-extent or screen-coordinate-overflowing rectangles
- rectangles outside fresh window outer bounds
- excessive advisory insertion position
- accepted capability inconsistent with target binding
- non-advancing generation

Invalid candidates leave the current generation exact.

## Target Resolution

| Input | Current checks | Result |
| --- | --- | --- |
| explicit `DropZoneId` | current lease, client epoch, live window, exact bounds, capability | one target or typed absence/ambiguity |
| `ScreenPoint` | containing live windows, current lease, exact bounds, eligible containing zones | one target or typed absence/ambiguity |
| overlapping live windows | no focus or order tie-break | ambiguous window |
| overlapping eligible zones | no array-order tie-break | ambiguous zone |
| changed outer bounds | leased geometry is stale | abort |
| expired lease | exact boundary check | abort |
| destroyed target window | lease removed | no target |

The explicit and screen-point paths return the same target evidence for the
same current zone. Direct panel targets bind a document, container, and region.
Hosted-window targets carry only opaque host and document evidence. Neither
path imports `longhorn-surfaces`.

## Package Report

| Package | Ordinary dependencies |
| --- | --- |
| `longhorn-transfer` | `longhorn-core`, serde |

The package imports no config, layout engine, Surface, window host, Tauri,
TypeScript, Svelte, Poodle, filesystem, clock provider, random provider, or
consumer authority.

## Validation

- `cargo test -p longhorn-transfer --quiet`: 13 passed
- `cargo clippy -p longhorn-transfer --all-targets -- -D warnings`: passed
- `cargo +1.85.0 check --workspace --all-targets`: passed
- `cargo tree -p longhorn-transfer --edges normal --depth 2`: core and
  serde only
- Surface-package and `SurfaceId` import scan: empty
- `effigy qa`: passed
- `effigy qa:northstar`: passed
- `effigy graph index --json`: passed with no diagnostics or failed paths
- `git diff --check`: passed

`effigy doctor` retains repository debt outside this card: one generated-source
warning and one high-size error in the concurrently developed Tauri lifecycle
model. It reports no `longhorn-transfer` file and no Card 031 core addition.

## Remaining Limits

- no authoritative panel mutation
- no whole-Surface mutation
- no target provisioner
- no Tauri handlers or managed-window projection
- no checked TypeScript or renderer client
- no client-CSS to screen-DIP projection adapter
- no packaged cross-window proof
- no donor migration

These remain assigned to Cards 032-035 and later adoption roadmaps.

## Posture

`strict-ready`

## Next

Start Card 032: bind consumed panel-transfer attempts to fresh same-document
layout authority and the existing expected-revision `MovePanel` publication.
