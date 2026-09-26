# 001 Working Rules

Status: active  
Owner: Tom  
Updated: 2026-09-26
Depends on: `../architecture/system-architecture.md`

## Delivery Grammar

- Knowledge in `docs/knowledge/` leads; intent lives in `docs/plan.md`; tasks,
  briefs, status and outcomes live in Queue.
- Promote durable shape and rules into architecture and contracts before
  execution.
- A brief defines outcome, governing refs, constraints, acceptance and stop
  conditions.
- Small recurring friction is filed in Queue as a papercut with
  `papercut.add` (payload in `AGENTS.md`), never in a repository file.

## Intent

- Stop when multiple plausible package boundaries require product priority.
- Stop when a consumer break needs operator policy.
- Do not mark a task ready while an intent checkpoint governs its scope.

## Refactoring

- Before v1.0, no compatibility aliases, deprecated stubs, re-export shims, or
  silent fallbacks.
- Coordinate material consumer breaks.
- Migrate call sites and remove superseded donor surfaces in the same bounded
  lane unless the operator chooses staged compatibility.

## Code Idioms

- Panic on a violated invariant with `expect("validated …")`, where the
  message names the guarding check. Bare `unwrap()` on an invariant is not the
  idiom — an auditor cannot tell it apart from an unexamined assumption.

## Definition Of Done

- real library behavior, not placeholder APIs
- at least one migrated consumer for an extraction lane
- dependent docs and fixtures current
- validation run and reported
- unresolved limits named

## Autonomy

- Bare `continue` is standing operator authority to begin the next item in
  `docs/plan.md` "Now".
- A planned task may move into execution when its governing refs, scope,
  evidence, and stop conditions are complete; it need not pause for repeated
  authorization.
- Stop on missing contracts, contradictions, failed evidence, or unclear
  consumer impact.

## Runtime

- Effigy first
- TypeScript with Bun for repo automation
- Bash only as thin glue
- other runtimes require a local reason

## Reporting

Lead with outcome, current state, then next move. Mention validation only
when it failed or changes confidence.

## Validation

- `effigy qa`
