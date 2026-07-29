# 018 Tauri Window Operation Execution

Status: complete
Owner: Tom
Roadmap: g01.004 batch 1
Governing refs: contracts 001, 003, 009, and 012; research memo 008
Auto-start next card: no

## Objective

Execute Card 016 operations against Tauri 2 with explicit managed identity,
consumer-owned creation, typed partial receipts, and convergence readback.

## Scope

- managed handle-to-`WindowId` registry
- explicit protected-primary slot
- retag as host bookkeeping
- injected dynamic `WebviewWindow` factory
- create, unmaximize, move/resize, maximize, show, hide, focus, and close
- main-thread dispatch
- apply generation registered before native calls
- per-operation success, failure, and dependency-skip receipts
- fresh complete live readback after apply
- capability derivation from available host and factory behavior

## Public Behavior

Creation asks a consumer factory for a neutral hidden unmaximized window. URL,
title, chrome, minimum size, capability pattern, and product metadata remain
factory policy. Returned handles must be unique and become managed only after
successful creation.

Execution is ordered and non-transactional. A failed operation blocks later
dependent operations for the same window. Independent windows continue.
Receipts retain generation, stable id, handle, operation, and typed Tauri error
context. Fresh readback decides convergence.

## Out Of Scope

- event attribution or persistence
- automatically changing desired state after user close
- creation defaults or Tauri capability-file generation
- retry, rollback, or fabricated atomicity
- layout, Surfaces, UI, or donor writes

## Steps

1. Add strict managed registry and protected-slot invariants.
2. Define the injected factory and neutral-result checks.
3. Define apply attempt, failure, skip, readback, and convergence receipts.
4. Dispatch an apply pass on the Tauri main thread.
5. Execute exact Card 016 ordering and geometry semantics.
6. Register generation and programmatic close evidence before calls.
7. Continue independent windows after failure.
8. Re-probe complete live state and report convergence.

## Acceptance Criteria

- labels never derive stable ids
- protected slot cannot be removed or closed by inference
- factory policy remains consumer-owned
- create result is hidden and unmaximized before later operations
- outer position and inner size map to distinct native calls
- retag/create precede geometry; close remains last
- native failure identifies operation, id, handle, and generation
- dependent operations skip after failure; independent windows continue
- intended success is never substituted for readback
- repeated apply over matching readback is empty

## Evidence Required

- protected retag and dynamic create fixtures
- every operation mapped to a host-call fixture
- factory failure and invalid-result fixtures
- move-success/resize-failure partial receipt
- per-window dependency skip and independent-window progress
- stale generation and programmatic-close evidence
- readback mismatch and idempotent convergence fixtures
- Rust 1.85 and full Effigy QA

## Stop Conditions

- creation requires shared product URL, chrome, title, or minimum policy
- apply must claim transactionality or rollback native state
- a failed operation must be ignored
- readback can be skipped while claiming convergence
- event, persistence, layout, or Surface state enters scope

## Next Task

Card 019 is ready against the exact planned-operation evidence, apply attempts,
and convergence readback implemented here. Review it before starting event
attribution.
