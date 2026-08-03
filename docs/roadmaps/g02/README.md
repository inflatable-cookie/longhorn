# g02 Workspace Integrity Remediation

## Generation Runway

| Milestone | State | Outcome |
| --- | --- | --- |
| [g02.001](001-layout-sizing-integrity.md) | complete | serde-proof layout ratio and sizing invariants |
| [g02.002](002-window-lifecycle-correctness.md) | complete | non-blocking event loop, coherent retag, safe install, timer wakes |
| [g02.003](003-transfer-session-truthfulness.md) | complete | truthful consumed-session aborts and race-free client bindings |
| [g02.004](004-host-thread-and-storage-coordination.md) | complete | off-main-thread storage commands and self-healing restore loads |
| [g02.005](005-injectable-diagnostics-seam.md) | complete | evidence for every best-effort failure swallow |
| [g02.006](006-qa-and-docs-alignment.md) | complete | resolving QA selectors, package hygiene, truthful front doors |

The runway is open-ended: g02 continues past remediation into whatever shared
gap consumer adoption characterizes next. Deferred candidates in the
[system inventory](../../architecture/system-inventory.md#planning-gaps)
remain uncommitted.

## Dependency Shape

```text
memo 018 workspace audit
 ├─ 001 layout sizing integrity
 ├─ 002 window lifecycle correctness ─┐
 ├─ 003 transfer session truthfulness ├─ 005 diagnostics seam
 ├─ 004 host thread and storage      ─┘
 └─ 006 QA and docs alignment
```

001-004 and 006 are independent. 005 lands after 002-004 settle the swallow
sites it instruments.

## Current Checkpoint

Research memo 018 promotes the post-g01 workspace audit. All findings sit
inside contracts 004, 010, 011, 012, 014, and 017; no new contract gates
execution. Cards 138-147 compile the six-milestone remediation runway.
All six remediation milestones and Cards 138-147 are complete. The
generation stays open for the next characterized shared gap.

## Consumer Guardrails

Remediation stays internal to Longhorn. No `packages/*/src/` file moves
(figmatic vite aliases), no crate or package add/remove (nucleus boundary
verifier), and the `notifications/operation` and
`tauri-transfer/surface-transfer` feature names stay fixed. Async command
migration must not change the invoke wire surface.

## Continuation

The [generation index](../generation-index.md) owns the only live next-task
pointer.
