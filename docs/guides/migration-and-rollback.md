# Migrate And Roll Back

Status: checked private adoption guidance
Updated: 2026-08-02
Governing contracts: [003](../contracts/003-extraction-and-consumer-migration.md),
[004](../contracts/004-configuration-storage-backup-and-recovery.md), and
[012](../contracts/012-distribution-and-compatibility.md)

## Why This Matters

Migration is where apps lose data — dual systems running in parallel, silent
fallbacks, deletion before rollback is proven. The rules below exist because
five apps (Nucleus, Loophole, Soundcheck, Split-shell, Jetstream) have already been
migrated, and every one of these failure modes has been hit. If you follow
only one thing from this guide: pick exactly one authority at bootstrap and
keep the old implementation frozen until the new one is proven.

## Admission

Migrate only behavior useful to two current apps or a stable mechanism with a
strong greenfield case. Product policy must remain separable. Similar UI is
not enough.

Before any consumer write, freeze:

- exact clean consumer, Longhorn, and Poodle commits
- behavior fixtures and native/renderer lifecycle traces
- current durable files, schemas, paths, locks, and migration inputs
- package, feature, peer, and capability graph
- authority map: old owner, new owner, retained product owner
- rollback source, tests, and cleanup restrictions
- unrelated dirty work and overlap proof

A source-linked graph is admitted only after the matching produced artifacts
install outside sibling workspaces.

## Cutover Sequence

1. Capture the old behavior and authority.
2. Map product policy to injected types, ports, and renderers.
3. Register stable ids and exact capabilities.
4. Import durable state backup-first into staged Longhorn-owned structure.
5. Run old fixtures and new conformance against copied/disposable state.
6. Select exactly one authority at bootstrap.
7. Move host protocol before optional presentation.
8. Prove fresh launch, restart, failure, recovery, teardown, and capability
   closure.
9. Remove the superseded donor mechanism in the same bounded migration.
10. Retain legacy durable source until the new publication and restart receipt
    authorize exact cleanup.

Do not keep old and new systems active as silent fallback. Do not dual-write
unless one explicit transaction authority can commit and reconcile both. A
donor copy left active after validated cutover is drift.

## Storage Transition

Never move config by changing a path constant. Use the fixed locator and
profile transition transaction. Discover legacy paths read-only, bind the
plan to exact source/target evidence, use native database adapters, commit the
locator last, and keep source bytes.

Cleanup is a later explicit action. It accepts only paths and digests in the
committed receipt, rechecks source and target under both authorities, and is
idempotent. “Migration passed” alone grants no deletion authority.

## A Concrete Example: Stable Storage Name

The storage guide describes profiles abstractly. Here is what a real
transition looks like for an app that moves from the default leaf to a stable
storage name:

| Step | Source (before) | Target (after) |
| --- | --- | --- |
| config | `~/Library/Application Support/com.example.product/config` | `~/Library/Application Support/Product/config` |
| data | `~/Library/Application Support/com.example.product/data` | `~/Library/Application Support/Product/data` |
| backups | `~/Library/Application Support/com.example.product/backups` | `~/Library/Application Support/Product/backups` |

The fixed locator stays at the canonical-id path and records “profile =
`platform-native-v1`, leaf = `Product`” — it is committed last, after the
files are staged and verified. Until the cleanup receipt authorizes it, both
trees exist and the old one is never touched by the new authority.

## Rollback Posture

Before cutover, rollback means leave the old authority selected and discard
unpublished staging.

After cutover but before legacy cleanup, rollback means select the retained
previous source and lock at a defined bootstrap boundary. It does not mean
running both systems or translating a partially committed live runtime back
in place.

After cleanup, rollback requires a separately proved previous-build readback
or restore path. Never claim rollback from source control alone when durable
state has changed.

Failure states stay distinct:

- rejected before publication: old authority remains exact
- new publication succeeded but activation failed: follow the domain receipt;
  do not repeat a non-idempotent mutation blindly
- restore rolled back: verified prior state is active
- recovery required: block normal writes and complete verified recovery
- native teardown partial: retain exact owner/handle evidence for explicit
  retry; do not report clean shutdown

## Lessons From Proven Consumers

| Consumer | Shared shape | Retained authority | Migration lesson |
| --- | --- | --- | --- |
| Nucleus | canonical storage, protected windows, Surface-free layout, child view | project semantics and browser/security policy | omit Surface; use project-keyed registered layout; preserve browser policy |
| Loophole | full display/window/Surface/layout/transfer/settings/commands/linear history | DAW/Pulse payload, journal, project version, focused-panel policy | decompose the hierarchy; keep history payload/apply/durability local |
| Soundcheck | stable-name storage, settings/recovery, operation lifecycle, isolated window | SQLite schema, scan reports, plugin/Signal policy | use native DB adapter and product-owned helper authorization |
| Split-shell | minimal config/settings | content, navigation, editorial and Git behavior | a split UI does not justify layout; preserve unrelated authored work |
| Jetstream | bridge/commands and backing-surface coordination | WGPU, renderer, world, input and execution | coordinate native geometry/lifecycle without absorbing engine authority |

Read the canonical [Nucleus](../architecture/nucleus-migration-map.md),
[Loophole](../architecture/loophole-migration-map.md), and
[secondary-consumer](../architecture/secondary-consumer-migration-map.md)
maps for exact receipts. Do not copy their product ids or policies into a new
app.

## Closeout

A migration closes only when shared implementation/tests pass, real consumer
authority is cut over, a materially different consumer or greenfield shape
conforms, duplicates are removed or explicitly retained as product policy,
exact artifacts install, restart and rollback pass, and docs/authority maps
are current.

Record unsupported platforms, unmet environment evidence, retained legacy
source, and deferred cleanup. Absence of evidence is not portability.
