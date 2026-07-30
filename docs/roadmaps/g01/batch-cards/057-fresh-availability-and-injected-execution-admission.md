# 057 Fresh Availability And Injected Execution Admission

Status: complete
Owner: Tom
Roadmap: g01.010 batch 1
Governing refs: contracts 006, 007, and 010; research memo 014
Depends on: Card 056
Auto-start next card: no
Completed: 2026-07-30

## Objective

Add current availability projection and authority-side execution admission over
the sealed registry, with consumer-injected context, availability, and typed
execution routes.

## Scope

- current context snapshot and monotonic context revision
- registry-generation-bound availability snapshots
- stable availability state and reason codes
- command request, admission, outcome, and bounded evidence types
- fresh context, capability, availability, and argument revalidation
- injected renderer-local and typed-domain executor ports
- cancellation, rejection, failure, and indeterminate outcomes
- stale renderer and route-mapping conformance fixtures

## Public Behavior

Renderer availability is a hint. Execution reloads current context facts,
checks the sealed registry generation, validates arguments, reruns capability
and availability admission, and only then calls a consumer executor.

The executor owns product authorization, product semantic validation, and the
actual route. It may call local behavior or one typed domain client. Longhorn
does not serialize a product command id into a generic bridge operation.

## Out Of Scope

- keyboard resolution or keymap storage
- generic Tauri command execution
- bridge operation registration
- product availability rules or receipts
- TypeScript, Svelte, Poodle, or settings

## Steps

1. Define bounded context snapshot and revision evidence.
2. Define availability states, coded reasons, and snapshot projection.
3. Define execution request, admitted invocation, and outcome taxonomy.
4. Add injected context, capability, availability, and executor ports.
5. Revalidate registry, arguments, and fresh current facts before dispatch.
6. Preserve bounded consumer evidence without interpreting product receipts.
7. Prove renderer-local and typed-domain route mappings.
8. Add stale registry, stale context, changed selection, lost capability,
   cancellation, failure, and indeterminate fixtures.
9. Audit bridge, Tauri, and product vocabulary absence.

## Acceptance Criteria

- registry generation and context revision are not interchangeable
- stale renderer availability never grants execution
- changed context or capability rejects before the executor runs
- product semantic rejection remains distinct from structural invalid input
- unknown, stale, unavailable, unauthorized, cancelled, rejected, failed, and
  indeterminate remain distinct
- local and typed-domain executors receive the same admitted invocation
- no shared API accepts a bridge route or arbitrary transport payload
- executor failure cannot mutate Longhorn registry or context state

## Evidence Required

- availability and execution outcome matrix
- executor call-count and non-call invariants
- stale context and capability race fixtures
- local versus typed-domain semantic trace
- bounded product-evidence fixtures
- bridge/Tauri/product-vocabulary audit
- focused Rust and Effigy checks

## Stop Conditions

- fresh admission depends on renderer state
- product authorization must move into the registry
- one generic bridge or Tauri execute endpoint is required
- an executor result cannot preserve indeterminate write posture
- command and bridge capability checks must be conflated

## Next Task

Card 058 is ready. Add deterministic physical-key and keymap resolution over
the same registry and ordered hot-context path.

## Result

`longhorn-command` now projects complete availability snapshots bound to one
registry generation and one distinct consumer context revision. Context paths
are finite, root-to-leaf, and revalidated against the sealed tree. Current
capability facts are canonical, bounded, and checked against registered
command capabilities.

Execution requests carry a bounded request id, observed registry generation,
semantic command id, and schema-checked arguments. Admission rejects stale
registries, unknown commands, invalid arguments, changed context, lost
capabilities, malformed fresh facts, and current product unavailability before
calling the executor.

Renderer availability never enters execution authority. The engine reloads
context, capability, and availability sources for each request. Only an
`AdmittedCommandInvocation` containing normalized arguments, the fresh context
revision, matched context, semantic command id, and opaque consumer route
reaches the injected executor.

Executor terminals preserve success, unauthorized, cancelled, rejected,
failed, and indeterminate posture. Consumer evidence is a stable opaque code
plus bounded optional text. Longhorn does not parse domain receipts. The
separate `admit` and `complete` seam supports asynchronous typed-domain calls
without adding an async runtime.

## Validation

- `effigy test:command-core`
- `cargo clippy -p longhorn-core -p longhorn-command --all-targets -- -D warnings`
- `cargo doc -p longhorn-command --no-deps`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-command-admission`
- 31 registry, availability, admission, route, outcome, serialization, and
  donor contract tests
- dependency audit: `longhorn-core`, Serde, `serde_json`, and SHA-256 only
- vocabulary audit: no bridge operation, bridge capability, Tauri, settings,
  renderer, Poodle, or donor type enters the pure crate
