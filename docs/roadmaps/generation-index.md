# Generation Index

## Mode

Dependency-led. New generations start only after a shared Longhorn gap is
characterized and promoted through the docs spine.

Northstar now has one execution level. The generation README owns the roadmap
and approved frontier; `docs/roadmaps/gNN/NNN-<slug>.md` is the sole executable
planning unit, referenced as `gNN.NNN`. There are no milestone wrappers and no
nested `batch-cards/` hierarchy: the flattened-task migration absorbed every
card record into its owning task (or, for Card 177, into new task g02.037) and
compacted g01 into `archive/g01.md`.

## Active Generation

[g02](g02/README.md) — workspace integrity remediation, now open past it.
Research memo 018 characterizes the post-g01 audit gap; Cards 138-147
delivered all six remediation tasks. Research memo 019 characterizes
the in-app update gap and compiles contract 018; g02.009 (Cards 150-154) is
the first non-remediation task. Research memo 020 compiles contract 019
over licensing as g02.010 (Cards 155-158).

## Rollover History

- [g01](archive/g01.md) — complete, 20 tasks, Cards 001-137. Roll-up replaces
  the removed expanded tree; git history is the full-fidelity archive.

## Next Task

**In delivery:** [g02.048](g02/048-agent-file-input.md) agent file input.
[g02.047](g02/047-rust-side-agent-selection.md) Rust-side agent selection
(PR #36) and [g02.049](g02/049-release-version-sweep.md) release version sweep
(PR #37) merged. Longhorn `0.2.1` published 2026-09-26 — tag `v0.2.1` at
`2605853b`, the three npm packages at `0.2.1` (`latest`).
The [g02 runway](g02/README.md#current-ready-frontier) is the single live
pointer. The release sequence below is history, not the current frontier.

1. [g02.014](g02/014-first-publication.md) first publication — **published
   2026-09-16**: the three TypeScript packages on npm, `v0.1.0` tagged at
   `7e81daa2`; consumer repoint outstanding
2. [g02.041](g02/041-staged-update-protocol.md) staged update protocol —
   complete, in `0.2.0`
3. [g02.042](g02/042-exclusive-admission-lease.md) exclusive admission lease —
   complete, in `0.2.0`
4. [g02.043](g02/043-packaged-agent-control-opt-in.md) packaged agent-control
   opt-in — complete, in `0.2.0`
5. [g02.044](g02/044-agent-control-stdio-carrier.md) agent-control stdio
   carrier — complete, in `0.2.0`

**Longhorn `0.2.0` published 2026-09-22** — tag `v0.2.0` at `df09c25a`, the
three npm packages at `0.2.0` (`latest`). It carries the staged update protocol
and exclusive admission lease (`g02.041`/`g02.042`), the packaged
`agent-control` opt-in (`g02.043`), the stdio carrier (`g02.044`), and the
Settings route-commit race fix. The first publish run went out partially (a
trusted-publisher mismatch on the two adapters, npm's 404 answer) and the
idempotent re-dispatch completed it; registry integrity matches the packed
artifact. What remains is the cross-repo consumer repoint and Desktop taking the
released artifact for the packaged private A→B proof (`g05.180`).

g02.020 and g02.026 completed on 2026-09-15 (PRs #27 and #28). g02.014's
Longhorn side is done; only the cross-repo consumer repoint remains. g02.041 and
g02.042 carry the operator-approved 2026-09-22 contract 018 amendment and ship
as Longhorn `0.2.0` (operator-owned release) for the `acowtancy` Desktop lane
`g05.179`. Parked and non-gating: g02.036 (L1 evidence and D1 blocked;
production held), g02.008 (deferred), g02.019 and g02.017 (Poodle/downstream,
field evidence). Task closeout detail lives on the tasks themselves.

## Memo 023 Release-audit Suite

Research memo 023 (full-repo release audit, 2026-08-14) compiles into
tasks [g02.021-027](g02/README.md) — cards 200-223, the suite that is
the live pointer above. Its state lives there; this section stays as the
memo-to-roadmap provenance.
