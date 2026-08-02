# API, Storage, Composition, And Migration Guides

Date: 2026-08-02
Roadmap: g01.016
Card: 126
State: complete; Card 127 ready

## Result

Longhorn now has one adoption front door covering package selection,
cross-platform storage and backup, system composition, migration and rollback,
and compatibility and upgrades. The guides derive claims from the contracts,
architecture records, consumer cutovers, and four produced-artifact greenfield
graphs. They do not require donor-code archaeology.

The checked API reference inventories all 36 Rust packages and 17 TypeScript
packages from repository metadata. It records features, source documentation,
public entrypoints, peer ranges, internal edges, and private-distribution
posture. Generation also found the child-view crate's missing `publish = false`
guard; the package now matches every other Rust package.

## Boundaries

- package-manager publication: false
- Rust packages: workspace paths or source archives only
- TypeScript packages: exact private tarballs only
- Poodle: public exports and components only
- canonical id is the default storage leaf; stable storage name is explicit
- profile transition is locator-last; legacy cleanup requires a receipt
- history shares linear structure, not product payload or project versions
- native-content claims remain mechanism- and platform-specific
- silent fallback, dual writes, and unreceipted cleanup remain forbidden

## Validation

- generated-reference drift check: pass
- local guide links and public-only snippets: pass
- Rust docs with warnings denied: pass
- TypeScript package checks: pass
- Svelte package checks: pass
- checked inventory: 36 Rust packages and 17 TypeScript packages

`effigy qa:northstar:g01-guides-card126` is the retained aggregate receipt.

## Next

Execute Card 127. Produce and verify one deterministic private `0.1.0`
candidate, then close g01.016 without publishing, tagging, or creating a hosted
release.
