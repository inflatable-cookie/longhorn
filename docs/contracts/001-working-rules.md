# 001 Working Rules

Status: active  
Owner: Tom  
Updated: 2026-07-27  
Depends on: `../architecture/system-architecture.md`

## Delivery Grammar

- Use `vision -> research/spec -> architecture + contracts -> roadmap ->
  execution -> evidence -> closeout`.
- Specs are provisional. Promote durable shape and rules before execution.
- A ready batch card defines scope, governing refs, steps, acceptance,
  evidence, stop conditions, and continuation.
- Roadmaps are multi-batch lanes. Cards carry step detail.
- Keep one live next-task pointer in roadmap front doors.

## Intent

- Stop when multiple plausible package boundaries require product priority.
- Stop when a consumer break needs operator policy.
- Do not mark a card ready while an intent checkpoint governs its scope.

## Refactoring

- Before v1.0, no compatibility aliases, deprecated stubs, re-export shims, or
  silent fallbacks.
- Coordinate material consumer breaks.
- Migrate call sites and remove superseded donor surfaces in the same bounded
  lane unless the operator chooses staged compatibility.

## Definition Of Done

- real library behavior, not placeholder APIs
- at least one migrated consumer for an extraction lane
- dependent docs and fixtures current
- validation recorded in a batch log
- unresolved limits named

## Autonomy

- Continue only across ready cards in the same valid lane.
- Stop on missing contracts, contradictions, failed evidence, or unclear
  consumer impact.

## Runtime

- Effigy first
- TypeScript with Bun for repo automation
- Bash only as thin glue
- other runtimes require a local reason

## Reporting

Lead with outcome, current lane state, then next move. Mention validation only
when it failed or changes confidence.

## Validation

- `effigy qa`
