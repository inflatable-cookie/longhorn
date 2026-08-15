# 221 Native-content Generation Hoist

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.027 batch 1
Governing refs: contract 017; memo 023 (M-native-content, L3 presentation
lane)
Depends on: none
Auto-start next card: no

## Objective

The attach-generation state machine contract 017 states once is implemented
once, in `longhorn-native-content`, with the three mechanism adapters
conforming to it.

## Why this exists

`validate_plan` is near-identical across
`longhorn-tauri-native-content-child-view/src/adapter/execute.rs:18-63`,
`longhorn-native-content-backing-surface/src/adapter/execute.rs:21-73`, and
`longhorn-native-content-isolated-window/src/adapter/execute.rs:21-67`; the
generation-compare helpers are character-identical modulo the error type. And
the copies have already drifted: backing-surface alone checks
`invalidated_generation` (`execute.rs:57`), isolated-window alone has
`FailedGeneration` re-attach semantics (`:48-54`). Contract 017 states the
rule once ("reject stale, future, retired, attaching, or absent generations",
`:79`); a rule fix currently has to land three times in three shapes.

Adjacent traps in the same files: the re-match `unreachable!()` at
backing-surface `execute.rs:110` re-matches the operation instead of binding
the matched `mode`, and the registry `expect("generation must be registered
before operation evidence")` panics
(`longhorn-tauri-windowing/src/apply/registry.rs:166`,
`longhorn-gpui-windowing/src/registry.rs:196`) sit where sibling misuse
returns typed errors.

## Scope

- `crates/longhorn-native-content` — the hoisted state machine
- the three mechanism adapters — conformance
- contract 017 — amended only if the hoist reveals the stated rule is not the
  implemented rule

## Steps

1. Extract the generation state machine into `longhorn-native-content` with
   the rule as contract 017 states it. Per-mechanism error enums stay; the
   state machine moves.
2. Reconcile the three known divergences against the contract: is
   `invalidated_generation` part of the rule (all adapters get it) or
   mechanism-specific (the contract says why)? Same for `FailedGeneration`
   re-attach. Each divergence resolves toward the contract or amends it.
3. Conform the three adapters; their conformance suites (which the audit
   found strong) are the safety net.
4. Fix the re-match `unreachable!` (bind the matched value) and give the
   registry invariants typed errors or documented `expect("validated …")`
   per the idiom Card 222 codifies — coordinate, don't duplicate.

## Do Not

- Fork the per-mechanism error types into one shared enum. The errors are
  mechanism-shaped on purpose; the rule is not.
- Let the hoist change observable behavior without the contract saying so.

## Result

The state machine lives once, in `longhorn-native-content`'s new
`generation` module: `GenerationRejection` (exactly contract 017's six
clauses), `AttachmentGate`, and the shared rule functions
(`compare_generation*`, `validate_plan_generation`, `check_attach_reservation`,
`gate_attached`, `gate_detach`). Adapters keep their error enums with total
`From` conversions; their util modules are thin wrappers. Nine new tests pin
the rule table; all three conformance suites pass unchanged.

Both known divergences resolved as **genuinely mechanism-specific**, and the
contract said less than the code — so contract 017 was amended
(`:119-128`) rather than the behavior unified:

- backing-surface's `invalidated_generation` exists because it invalidates on
  host destroy *before* its reversible detach settles; folding it into the
  shared gate would have flipped observable error variants.
- isolated-window's `FailedGeneration` re-attach exists because only it has
  an owner process that can die while the island lives on.

The re-match `unreachable!` now binds the matched mode. The Tauri registry's
invariant panic became a typed `EvidenceBeforeGeneration` error; the GPUI one
keeps the panic under the named-guard idiom (its signature is infallible).

## Acceptance Criteria

- [x] a generation-rule fix lands in one file
- [x] all three adapters' conformance suites pass against the shared machine
- [x] each known divergence is resolved toward or amended into contract 017
- [x] the two panic traps are typed or documented

## Evidence Required

- the hoist diff and the three suites green
- the divergence resolutions recorded
- `effigy qa` green

## Stop Conditions

Stop if the hoist shows the three mechanisms genuinely need different state
machines — that contradicts contract 017's single statement and is a contract
conversation, not a refactor.
