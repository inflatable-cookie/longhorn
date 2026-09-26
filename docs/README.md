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
<!-- northstar:lifecycle:begin schema=northstar.lifecycle.projection.v2 digest=sha256:6b4599a38528a753887119e8ded0889e94a73bd892cf07cec8e1bcb3dc77e96d -->
| Generation | Disposition | Runway state |
| --- | --- | --- |
| g02 | open | planning_required |
| Task | Status | Stage | Revision | Record digest |
| --- | --- | --- | --- | --- |
| g02.020 | complete | none | 8 | sha256:070656c53d6870a5fbe4ad0f330aadcdaa493f9a584a3590442cb557d924213e |
| g02.026 | complete | none | 8 | sha256:c3cc3ac84dab66ea08d83fbd8695d6f9cec6cc363d50e756d94bc1b9a73f6780 |
| g02.038 | complete | none | 8 | sha256:7a76e636abf326e7ce7391e8f18279e6e8792d51686c82ec34f97ca1a1350855 |
| g02.039 | complete | none | 8 | sha256:f37506b19afd8705484d9c4e270c5fd42b8faf6feb3533ddac9a47cb183dc6a4 |
| g02.040 | complete | none | 8 | sha256:057c72544bb197dfbb649330eb0bbd17561e911f43d39ea118fe70c75486dc46 |
| g02.041 | complete | none | 8 | sha256:365665d50017539b80c8fd719c4690b63964bb5cd61dcded409ad4d1bf77c55d |
| g02.042 | complete | none | 8 | sha256:312260b4fae2df0cbe6cebbcbc278af676d1814f73449b28acd5b8e82455216c |
| g02.043 | complete | none | 8 | sha256:c75dbb8b2ca27dd01e5d14b8e9399cc492ee6c06b49d832207374b14d3f99d91 |
| g02.044 | complete | none | 8 | sha256:0f217dc0485674ec3da54ab81c584c97b9f83ca8003b530268dd90fb2f428f7c |
| g02.045 | complete | none | 8 | sha256:8f906b02c4484739d9905aaf428177a5cbb9662312152c118a36437d486222d7 |
| g02.046 | complete | none | 8 | sha256:4ea21290bbb99dc3a8ef68262990d2d8c491b25f20268d77d70c37a5c8b00539 |
| g02.047 | complete | none | 8 | sha256:66c70f20d85e7d9a204418fed81da34fe36ed695437e0df88685493bf46a4239 |
| g02.049 | complete | none | 8 | sha256:33f50a9d46585a3e41b66d017be5fbd1a72ac72cc4d179ac12b47bd0f03fb18f |
<!-- northstar:lifecycle:end -->
