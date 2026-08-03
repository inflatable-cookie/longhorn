# g02.005 Injectable Diagnostics Seam

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 010, and 012; research memo 018
Depends on: g02.002, g02.003, g02.004

## Outcome

Give every best-effort failure swallow a diagnostic seam — an injectable
diagnostics callback (or optional `tracing` feature) — so event-emit,
teardown, wake, and cleanup failures leave evidence without becoming errors.

## Generation Runway

Fifth g02 milestone. Runs after 002-004 settle which swallow sites survive.
Additive and optional; default behavior stays silent-tolerant.

## Execution Plan

### Batch 1. Seam and instrumentation

- [x] [Card 145](batch-cards/145-diagnostics-seam-and-swallow-instrumentation.md)
  defines the seam and instruments the audited swallow sites across the
  tauri-* command layers, native-content adapters, and restore cleanup

## Goals

- [x] one seam shape reused across crates; no new hard dependency
- [x] every audited `let _ =` site routed through it
- [x] zero behavior change when no diagnostics are injected

## Acceptance Criteria

- [x] injected-diagnostics tests observe emit, teardown, wake, and cleanup
  failures
- [x] default composition compiles and behaves identically; workspace QA
  passes
- [x] nucleus boundary verifier unaffected (no crate/package add)

## Explicit Non-goals

- mandatory logging or a global subscriber
- consumer repository edits

## Next Task

Promote Card 146 when Card 145 closes (146 may run earlier; it is
independent).
