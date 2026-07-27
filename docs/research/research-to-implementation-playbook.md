# Research To Implementation

Status: active  
Owner: Tom  
Updated: 2026-07-27

## Discovery

1. Name the architecture or contract question.
2. Read `master-index.md`.
3. Inspect current consumer code and tests.
4. Treat dirty working-tree code as evidence, not stable authority.

## Decision

Record:

- sources consulted
- recommendation and tradeoffs
- rejected donor-specific behavior
- prototype or contract gaps

Promote execution-relevant structure into architecture and behavior into
contracts before roadmap compilation.

## Implementation

- reference governing research where behavior is non-obvious
- capture new gaps instead of improvising
- stop when implementation contradicts promoted authority
- modify consumer repos only from an explicit migration card

## Validation

- derive shared fixtures from donor behavior
- test both full and reduced compositions
- record packaged native proofs where mock Tauri cannot establish behavior

## Review

- donor facts still accurate
- consumer policy stayed outside Longhorn
- deviations explicit
- authority transferred only after cutover evidence
