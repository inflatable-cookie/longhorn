# Fresh Availability And Injected Execution Admission

Date: 2026-07-30
Card: 057
Roadmap: g01.010

## Result

Extended `longhorn-command` with pure current-context, capability,
availability, fresh admission, and injected execution seams. Renderer
availability remains advisory. Execution has no API that accepts a renderer
snapshot as authority.

## Fresh Facts

- `CommandContextRevision` is distinct from registry generation
- one bounded root-to-leaf hot-context snapshot begins at `global`
- deserialization routes through the same checked context constructor
- capability facts are bounded, deduplicated, ordered, and registry-checked
- availability snapshots name registry generation and context revision
- every availability record is in stable command-id order
- available, unavailable, hidden, and unsupported states are explicit
- every non-available state carries a stable built-in or consumer reason
- optional availability and outcome detail is nonempty and bounded to 4 KiB

## Admission Order

1. compare the caller's registry generation
2. resolve the semantic command id
3. structurally validate and normalize arguments
4. reload and validate the current hot-context path
5. reload and validate current command capabilities
6. rerun registered context and capability admission
7. rerun consumer-owned product availability
8. call the consumer executor with one admitted invocation

Stale, unknown, invalid, unavailable, hidden, unsupported, or failed-source
paths do not call the executor. A stale renderer snapshot cannot be supplied
to this sequence.

## Outcomes

The request-correlated result distinguishes:

- unknown command
- stale registry
- invalid structural arguments
- unavailable current facts
- unauthorized product execution
- cancellation
- product semantic rejection
- definite failure with phase
- indeterminate authoritative effect
- success

Opaque consumer evidence contains only a bounded stable code and optional
bounded text. `admit` plus `complete` lets an asynchronous consumer await a
typed domain operation outside Longhorn. The synchronous `execute` helper
uses the same admitted invocation and terminal mapping.

## Route Proof

A renderer-local executor and a test-only typed editor-domain executor receive
the exact same admitted invocation. The domain executor maps the opaque
consumer route to a typed operation after admission. No command id becomes a
bridge or Tauri operation name.

## Failure Matrix

Fixtures prove no executor call for:

- stale registry, unknown command, or structurally invalid arguments
- changed hot context after an available renderer snapshot
- lost required capability after an available renderer snapshot
- product-owned unavailable posture
- context, capability, or availability source failure
- malformed current context topology
- unregistered current capability facts

Executor failure leaves the immutable registry digest and supplied fresh
context facts unchanged. Indeterminate remains distinct from definite failure.

## Boundary Audit

- normal dependencies remain `longhorn-core`, Serde, `serde_json`, SHA-256
- no async runtime, config, settings, bridge, Tauri, renderer, Svelte, Poodle,
  or donor dependency
- `serde_json::Value` exists only at the closed structural argument boundary
- executor input contains normalized arguments and `CommandRouteId`, never an
  arbitrary transport payload
- command capabilities remain distinct from bridge capability and authority

## Validation

- `effigy test:command-core`
- 31 command contract tests plus core tests and doc tests
- `cargo clippy -p longhorn-core -p longhorn-command --all-targets -- -D warnings`
- `cargo doc -p longhorn-command --no-deps`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-command-admission`
- `effigy qa:northstar`

## Next

Card 058 is ready. Add deterministic physical-key and keymap resolution without
adding renderer order, browser text, native accelerator, or persistence
authority.
