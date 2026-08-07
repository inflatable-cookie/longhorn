# Generation Index

## Mode

Dependency-led. New generations start only after a shared Longhorn gap is
characterized and promoted through the docs spine.

## Active Generation

[g02](g02/README.md) — workspace integrity remediation, now open past it.
Research memo 018 characterizes the post-g01 audit gap; Cards 138-147
delivered all six remediation milestones. Research memo 019 characterizes
the in-app update gap and compiles contract 018; g02.009 (Cards 150-154) is
the first non-remediation milestone.

## Rollover History

- [g01](g01/README.md) — complete, 20 milestones, Cards 001-137

## Next Task

[Card 150](g02/batch-cards/150-store-schema-stamping-and-forward-refusal.md)
is complete: every store's future-schema refusal is now proved
non-destructive, and one shared classification in `longhorn-core` lets the
update surface recognise a channel rejoin without matching per-store errors.

[Card 151](g02/batch-cards/151-update-policy-channels-and-rollout.md) is
ready and does not auto-start. It builds `longhorn-update` — channel
resolution, semver comparison, client-side rollout, deferral — with no Tauri
dependency.

The v0.1.0 tag is blocked on a poodle release; g02.009 is independent of it.

Also outstanding: Card 149's receipt freeze remains operator-held on
consumer manifest quiescence, and the
[g02 candidate runway](g02/README.md#candidate-runway) still tiers the
deferred options. Package-manager publication remains deferred.
