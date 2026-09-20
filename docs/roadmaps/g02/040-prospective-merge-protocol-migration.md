# g02.040 — Prospective-merge protocol migration

Owner: repository maintainers
Created: 2026-09-21
Governing refs: Northstar g03.020; Queue Spec 015
UI classification: none

## Outcome

Longhorn's Queue control manifest uses `paseo.queue.control.v4` and evaluates
the required pre-merge hook against the exact prospective merge candidate
instead of the reviewed head.

## Ready-State Rubric

- [x] Northstar g03.020 is terminal at `f34e1c0`.
- [x] The installed public migration dry-run accepts this repository exactly.
- [x] Tom authorized the portfolio rollout on 2026-09-16.

## Dispatch manifest

- **State:** ready; configuration maintenance independent of product work.
- **Owned mutable paths:** `.paseo/queue.json` only.
- **Worker:** automatic mechanical/general pool with independent review.
- **Excluded:** product code and planning, dependencies, releases, CI, Queue
  state and thread/workspace disposition.

## Work

1. Run the installed `northstar/lifecycle:migrate-premerge` dry-run.
2. Apply the exact accepted migration with `-- --write`.
3. Prove the diff changes only `.paseo/queue.json`, moving the schema from v3 to
   v4 and the pre-merge target from `reviewed_head` to `prospective_merge`.
4. Validate the manifest and run the repository's focused checks.

## Acceptance and review oracle

Write mode reports `applied`, replay reports `unchanged`, the v4 manifest is
valid, and no file except `.paseo/queue.json` changes.

## Stop conditions

Stop on divergent input, dirty base, extra changed paths, missing installed
command or validation failure. Never hand-edit around a refusal.

## Next task

Return to the existing project frontier. This task authorizes no product
successor.
