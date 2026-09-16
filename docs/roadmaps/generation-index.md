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

**Release sequence (operator direction 2026-09-15):** finish and unblock the
existing lanes, then cut the first release. The runway lives on the
[g02 runway](g02/README.md#release-sequence); this is the single live pointer.

1. [g02.014](g02/014-first-publication.md) first publication — **published
   2026-09-16**: the three TypeScript packages on npm, `v0.1.0` tagged at
   `7e81daa2`; consumer repoint outstanding

g02.020 and g02.026 completed on 2026-09-15 (PRs #27 and #28). g02.014's
Longhorn side is done; only the cross-repo consumer repoint remains. Parked
and non-gating: g02.036 (L1 evidence and D1 blocked; production held), g02.008
(deferred), g02.019 and g02.017 (Poodle/downstream, field evidence). Task
closeout detail lives on the tasks themselves.

## Memo 023 Release-audit Suite

Research memo 023 (full-repo release audit, 2026-08-14) compiles into
tasks [g02.021-027](g02/README.md) — cards 200-223, the suite that is
the live pointer above. Its state lives there; this section stays as the
memo-to-roadmap provenance.
