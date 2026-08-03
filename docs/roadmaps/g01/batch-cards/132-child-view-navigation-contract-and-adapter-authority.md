# 132 Child-view Navigation Contract And Adapter Authority

Status: complete
Owner: Tom
Roadmap: g01.020 batch 1
Governing refs: contracts 001, 003, 012, and 017; Figmatic g03.006
Depends on: g01.018
Auto-start next card: yes
Completed: 2026-08-03

## Objective

Freeze and implement exact adapter-local navigation authority for one retained
child generation.

## Scope

- promoted child-view navigation boundary
- generation-checked current-URL observation
- consumer-policy admission before native work
- unchanged/submitted receipts
- page-load readiness transitions
- denial, stale, absent, observation, and native failure evidence

## Out Of Scope

- common native-content operation or renderer protocol changes
- browser-history actions
- packaged or consumer-repository proof

## Steps

1. Promote the Figmatic evidence into contract and architecture.
2. Add native-side navigation outcome and receipt types.
3. Extend the runtime port with current URL and navigate operations.
4. Enforce generation and policy before native observation or mutation.
5. Make same-URL requests unchanged and reset readiness on load start.
6. Prove the complete failure and retained-handle matrix with a fake runtime.

## Acceptance Criteria

- the common operation enum and generated renderer protocol do not change
- denied and stale requests perform no native URL read or mutation
- one allowed different URL produces exactly one native navigation
- a repeated current URL produces no native navigation
- navigation never advances or recreates the attach generation
- load start makes the current generation not ready; finish makes it ready

## Evidence Required

- contract and architecture diff
- focused Rust tests and Clippy
- exact fake-runtime call trace
- common-protocol drift check

## Stop Conditions

- correct execution requires URL state in the pure coordinator
- policy cannot be evaluated before native mutation
- Tauri cannot navigate the retained opaque handle

## Next Task

Card 133 adds the real Tauri operation and packaged retained-navigation proof.

## Evidence

- contract 017 and native-content composition keep URL state out of the common
  operation and renderer protocol
- the public adapter exposes generation-checked current URL plus unchanged or
  submitted navigation receipts
- 10 fake-runtime tests cover retained identity, policy denial, stale/future
  authority, same-URL no-op, readiness transitions, and native failures
- `effigy qa:native-content-child-view`, the dependency graph check, and the
  native-content binding drift check pass
