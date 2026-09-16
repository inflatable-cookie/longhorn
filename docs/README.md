# Longhorn Docs

The documentation for Longhorn, a pre-1.0 workspace of shared Rust and
Svelte/TypeScript systems for Tauri desktop apps. The TypeScript packages
publish to npm; the Rust crates are taken by git tag. This repo records both
how the systems work and how they were built.

## Start Here

For new readers, in order:

1. [Vision](vision/README.md) — what Longhorn is and why it exists
2. [Adoption Guides](guides/README.md) — how to choose and integrate packages
3. [Glossary](guides/glossary.md) — the terms the docs use
4. [API Reference](reference/README.md) — exact packages, crates, entrypoints
5. [Architecture](architecture/README.md) — how the systems are structured
6. [Contracts](contracts/README.md) — the rules the systems must follow

For maintainers and agents:

- [Research](research/README.md) — the audits and boundary memos behind the design
- [Specs](specs/README.md) — provisional planning before promotion
- [Roadmaps](roadmaps/README.md) — delivery planning, including the
  [generation index](roadmaps/generation-index.md) with the live next-task pointer
- [Logs](logs/README.md) — batch evidence of completed work
- [Triage](triage/README.md) — open observations awaiting promotion or disposition

## Current State

Pre-1.0 workspace. Generation g01 is complete (20 tasks, cards 001-137, five
migrated consumers; roll-up at
[roadmaps/archive/g01.md](roadmaps/archive/g01.md)). Generation g02 is active
as one executable task per `g02.NNN` file, across application-update and
licensing runways, the memo-023 release-audit suite (tasks g02.021-027), and
the agent-control arc through g02.035, with consumer-scoped credential slots
at g02.028.

`0.1.0` published 2026-09-16 spans the TypeScript packages and Rust crates
enumerated — always currently — by the generated
[API reference](reference/api-surface.md). The three TypeScript packages are
on npm under `@inflatable-cookie`; the Rust crates stay off crates.io and are
taken by git tag `v0.1.0`.

The complete delivery history is preserved in the
[roadmap generation index](roadmaps/generation-index.md).
<!-- northstar:lifecycle:begin schema=northstar.lifecycle.projection.v2 digest=sha256:36e33b13405a246aba97f4283fd48a8b37bd1cfd7a77ea523e42c1a0d04e34b9 -->
| Generation | Disposition | Runway state |
| --- | --- | --- |
| g02 | open | planning_required |
| Task | Status | Stage | Revision | Record digest |
| --- | --- | --- | --- | --- |
| g02.020 | complete | none | 8 | sha256:070656c53d6870a5fbe4ad0f330aadcdaa493f9a584a3590442cb557d924213e |
| g02.026 | complete | none | 8 | sha256:c3cc3ac84dab66ea08d83fbd8695d6f9cec6cc363d50e756d94bc1b9a73f6780 |
| g02.038 | complete | none | 8 | sha256:7a76e636abf326e7ce7391e8f18279e6e8792d51686c82ec34f97ca1a1350855 |
| g02.039 | complete | none | 8 | sha256:f37506b19afd8705484d9c4e270c5fd42b8faf6feb3533ddac9a47cb183dc6a4 |
<!-- northstar:lifecycle:end -->
