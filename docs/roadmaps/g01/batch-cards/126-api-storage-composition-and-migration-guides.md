# 126 API, Storage, Composition, And Migration Guides

Status: ready
Owner: Tom
Roadmap: g01.016 batch 5
Governing refs: contracts 001-017; Card 125
Depends on: Card 125
Auto-start next card: no

## Objective

Document the proven public APIs and composition decisions so a greenfield app
or existing consumer can adopt Longhorn without reconstructing donor history.

## Repository Scope

- Longhorn: rustdoc, TypeScript API docs, guides, examples, indexes, and docs QA.
- Consumer repositories: read-only source-linked examples and migration receipts.
- Registries and hosted docs: no publication.

## Scope

- package selection and optional dependency matrix
- canonical id, stable storage name, profiles, roots, domains, and filesystem locations
- safe writes, backups, restore, recovery, encryption, and source cleanup
- window/layout/Surface composition and transfer
- settings registration, commands/keymaps/palette, bridge topology, linear history,
  operations, notifications, and native content
- consumer migration, exact private artifacts, rollback, and compatibility failures
- unsupported/deferred features and platform claims

## Steps

1. Generate and check Rust and TypeScript API references from public surfaces.
2. Write a package-selection guide keyed to the four examples.
3. Write one cross-platform storage/configuration/backup guide with exact paths.
4. Write composition guides for local, workspace, full-hosting, and optional-server apps.
5. Write migration and rollback guidance from Nucleus, Loophole, and secondary consumers.
6. Write compatibility, versioning, protocol, and upgrade guidance.
7. Cross-link examples, contracts, errors, and unsupported-platform statements.
8. Audit every snippet against produced artifacts and public Poodle APIs.

## Acceptance Criteria

- a new app can choose a composition without reading donor code
- filesystem examples distinguish canonical id, display name, and stable storage name
- macOS, Windows, and Linux profile defaults and lifecycle roots are exact
- backup and restore docs state adapter consistency and recovery limits
- history docs distinguish linear structure, product payload, project versions, and deferred tree
- native-content docs state mechanism-specific platform support
- migration docs forbid silent fallback, dual writes, and unreceipted cleanup
- no guide implies package-manager availability

## Stop Conditions

- generated docs expose private or consumer types
- a snippet needs an internal Poodle or Longhorn path
- a support claim lacks artifact or consumer evidence
- docs describe publication as complete

## Next Task

Execute Card 127. Produce one deterministic private `0.1.0` candidate, run
read-only release gates, and close g01.016 without publishing.
