# Held Surface

Status: checked
Updated: 2026-08-08

Some Longhorn surface is built, tested, and contract-documented but not yet
ready for consumer implementation. This register is the single source of
truth for what is held and why. "Held" never means deleted or half-wired:
the code stays, the docs say it stays, and the trigger below says what moves
it out of this register.

Statuses:

- `held-for-consumer` — built against the contract, awaiting a named
  consumer or card to implement against.
- `planning-evidence` — proved, but no consumer or card is named yet.

Rules enforced by `scripts/verify-held-surface.ts`:

1. Every entry names a crate or package that exists and appears in the
   [generated API surface](api-surface.md).
2. Held entries are never presented as selectable in the
   [adoption guides](../guides/README.md); the guides link here instead.
3. Moving a surface to selectable is a doc + gate change in one batch.

## Register

| Surface | Status | Awaits | Trigger |
| --- | --- | --- | --- |
| age encryption (`longhorn-config-age`, no `bindings` feature) | held-for-consumer | a consumer that needs encrypted backups | contract 004 already documents optional age v1; integration seam = `ConfigStore` capture/restore hooks plus archive inspection. Until then, composition is the documented two-call flow: encrypt via config-age, decrypt and inspect via config-age, then hand plaintext to `longhorn-config`. |
| application update crates (`longhorn-update`, `longhorn-update-install`, incl. their `bindings` features) | held-for-consumer | the update renderer runway (g02.009) | `packages/update` card (154) plus packaged proof (159). The restart interlock, gate, installer and provenance classification are implemented and tested; no renderer surface exists. `longhorn-tauri-update` was absorbed into `longhorn-update` on 2026-08-09 and `longhorn-update-native` renamed to `longhorn-update-install`. |
| licensing (`longhorn-licence`, incl. its `bindings` feature) | held-for-consumer | the licensing renderer runway (g02.010) | `packages/licence` card (158) plus the Tauri licence host card (157). Rust-only today. |
| bridge contract machinery (`longhorn-bridge`: lifecycle machine, replay ledger, ordering, supervision, jobs) | held-for-consumer | a bridge consumer with a real service topology | the optional-server shape needs supervision or ordered delivery. Until then the surface is exercised by the crate's own contract tests; supervision is a stub on both sides of the renderer boundary. |
| fork-tree optional surface (`longhorn-history-tree` branch clients, checkpoints, dense persistence; `@inflatable-cookie/longhorn/history-tree` fork projections) | planning-evidence | the recorded g01.017 adoption decision | the history runway promotes the tree when a consumer needs divergent history. The linear slice is the shipped surface; the tree crate is selectable only as evidence. |
| layout bounded replay (`BoundedLayoutReplayStore`, `apply_with_replay`) | planning-evidence | a host that needs replay at the mutation boundary | contract 014 sanctions it as host-opt-in; currently exercised by contract tests only. |
| `MilestoneRetention` (backup retention age buckets) | planning-evidence | a retention product choice | contract 004 documents count/age tiers; milestone buckets are uncontracted. Contract or retire with the retention consumer. |

## Assessed And Retained

The config crate's unexported-surface trim was attempted (2026-08-08) and
reverted: five free functions (`discover_legacy_storage`,
`plan_storage_transition`, `apply_storage_transition_cleanup`,
`resolve_storage_bootstrap_paths`, `encode_backup_export_archive`) have zero
internal and zero external callers — they are fully dead public API, not
ConfigStore-internal helpers. Narrowing them to `pub(crate)` surfaces
dead-code lints, and deleting them would remove documented migration
behavior. They remain public, unused, and harmless in a private workspace.
Decision deferred: contract them with a real caller, or delete when the
retention/transition runway next touches them.
