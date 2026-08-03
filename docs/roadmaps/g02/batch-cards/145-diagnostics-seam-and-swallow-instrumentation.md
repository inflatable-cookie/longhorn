# 145 Diagnostics Seam And Swallow Instrumentation

Status: complete
Owner: Tom
Roadmap: g02.005 batch 1
Governing refs: contracts 001, 010, and 012; research memo 018
Depends on: Cards 139-144
Auto-start next card: no
Completed: 2026-08-03

## Objective

Define one injectable diagnostics seam and route every audited best-effort
failure swallow through it.

## Scope

- seam definition (shared shape; no new crate, no mandatory dependency)
- swallow sites in `longhorn-tauri-{operation,history,history-tree,
  notifications,windowing}` command/event layers
- native-content adapter teardown paths
- restore journal cleanup

## Steps

1. Choose the seam: injectable diagnostics callback on existing composition
   types, or an optional `tracing` feature — decide against workspace
   dependency posture and record the decision in the card log.
2. Instrument the memo-018 swallow inventory; default composition stays
   silent-tolerant with zero behavior change.
3. Add injected-diagnostics tests observing emit, teardown, wake, and cleanup
   failures.
4. Audit that no `let _ =` on a fallible best-effort path remains
   uninstrumented; record any deliberate exceptions.

## Acceptance Criteria

- one seam shape reused across crates; no crate/package count change
- all audited sites instrumented or recorded as deliberate
- default behavior identical; workspace QA passes

## Evidence Required

- seam decision record
- instrumentation coverage list against memo 018
- injected-observation test receipts and QA receipts

## Stop Conditions

- the seam cannot avoid a public composition-API break for existing
  consumers

## Evidence

- `longhorn-core` seam (trait + install + report), first-install-wins,
  silent default; seam test proves capture and precedence
- 23 sites instrumented; zero unjustified production swallows remain;
  deliberate non-sites recorded
- log: `docs/logs/2026-08/03-diagnostics-seam-and-swallow-instrumentation.md`

## Next Task

Promote Cards 146-147 (g02.006).
