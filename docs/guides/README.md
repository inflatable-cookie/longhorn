# Adoption Guides

These guides explain how to select Longhorn packages and integrate them into a
Tauri app. Longhorn remains a private coordinated `0.1.0` workspace: the
guides work from produced artifacts, and they do not claim npm, crates.io,
hosted documentation, tags, or public releases.

New here? Read the [glossary](glossary.md) first — the docs use terms like
authority, receipt, and donor in precise ways.

## Start Here

1. [Getting Started](getting-started.md) — zero to running, in one pass
2. [Choose Packages](package-selection.md)
3. [Storage, Configuration, Backup, And Recovery](storage-configuration-backup.md)
4. [Compose Desktop Systems](system-composition.md)
5. [Migrate And Roll Back](migration-and-rollback.md)
6. [Compatibility And Upgrades](compatibility-and-upgrades.md)
7. [Glossary](glossary.md)

Exact package names, features, entrypoints, dependencies, and peers are in the
[generated API surface](../reference/api-surface.md). Symbol-level behavior
lives in package READMEs and checked Rust documentation.

The [greenfield examples](../../examples/greenfield-compositions/README.md)
are neutral build roots. Donor migration maps are evidence, not application
templates.
