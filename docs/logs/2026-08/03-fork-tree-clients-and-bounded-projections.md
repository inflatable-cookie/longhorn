# Fork-tree Clients And Bounded Projections

Date: 2026-08-03
Card: 073
Roadmap: g01.017

## Result

The optional tree now has metadata-only clients without changing linear
packages or carrying product payloads:

- pure linear-default summary and preferred path page
- explicit stable-id branch pages and explicit branch-path pages
- shared 256-record hard page ceiling, offsets, totals, and truncation
- exact-v1 Rust-generated TypeScript protocol and golden fixture
- payload-free revision-bound navigation commands, receipts, rejections, and
  non-durable invalidation hints
- caller-aware `longhorn-tauri-history-tree` mock-runtime host
- checked direct, serialized-loopback, and Tauri ports
- listener-first per-instance Svelte sessions and public-Poodle panel

## Default And Alternate Loading

Startup calls `listen`, `snapshot`, then one default path page. It does not
request branch metadata or alternate paths. Summary reports branch and leaf
counts without allocating their records. `loadBranches` is explicit. Selecting
one branch requests only that bounded branch-head path. A 2,048-node fixture
returns exactly the requested 17 records.

## Failure And Lifecycle Evidence

- commands bind exact protocol, authority epoch, history id, and graph revision
- stable branch and entry ids are navigation authority; presentation indexes
  never cross the protocol
- future protocol, unknown fields, and any payload-named field reject
- stale result installation is epoch/revision checked
- listener registration completes before snapshot load
- late async listener registration disposes exactly once after stop
- mounted teardown removes listeners; SSR import touches no browser globals
- committed navigation publishes a hint; rejection remains non-durable and
  emits no false commit

## Boundary Audit

The Tauri crate imports only tree history, serde, and Tauri. The TypeScript
root depends only on `@longhorn/core`; Svelte and Poodle remain optional
subpaths and peers. The Poodle adapter imports public Button, Callout,
InlineListSection, Spinner, and Stack primitives. No Poodle source or internal
selector is copied. Product payloads, checkpoints, storage, durability,
project versions, collaboration, merge, and event sourcing remain absent.

## Validation

- 26 pure tree tests
- 4 Tauri host/capability tests including the mock runtime
- 9 direct/serialized/controller/Svelte/Poodle/SSR tests
- checked binding regeneration, TypeScript, Svelte, package, Clippy, and Rust
  1.85 paths

## Next Task

Execute Card 074. Prove isolated linear-only and tree-enabled artifacts and
close g01.017.
