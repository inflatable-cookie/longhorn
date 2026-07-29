# 030 Surface And Window Host Composition Conformance

Status: complete
Owner: Tom
Roadmap: g01.006 batch 2
Governing refs: contracts 001-004, 009, 012, and 014; research memo 010
Depends on: Cards 028-029 and completed g01.004
Auto-start next card: no

## Objective

Compose resolved Surfaces with the existing pure window plan and runtime-generic
window host, then prove full and no-Surface dependency shapes.

## Scope

- Surface resolution to desired participating-window bindings
- existing window placement, protected-primary, readiness, and teardown seams
- consumer-injected window roles, factory policy, and presence input
- missing-window and temporary-fallback behavior
- Loophole full hierarchy fixture
- Nucleus direct-window fixture and dependency exclusion
- mock host reconciliation and failure receipts
- Surface persistence flush before bounded shutdown

## Public Behavior

Surface resolution supplies host bindings; `longhorn-windowing` still owns
placement and live diff. The consumer supplies window roles, URLs, titles,
capabilities, creation policy, and product presence.

Missing windows do not rewrite preferred hosting. Native apply is
nontransactional and receipted. Durable Surface state changes only through
Card 029 authority.

## Out Of Scope

- cross-window drag sessions
- empty-display provisioning
- generated TypeScript or reusable renderer clients
- Svelte/Poodle adapters
- packaged drag proof
- donor migration

## Steps

1. Define the narrow Surface-to-window desired-binding projection.
2. Compose it with existing placement and diff inputs.
3. Inject consumer window factory and policy without product types.
4. Preserve protected-primary and readiness behavior.
5. Reconcile missing, returning, and temporarily unavailable windows.
6. Wire explicit Surface flush into bounded host shutdown.
7. Add mock apply, partial failure, retry, and teardown fixtures.
8. Add Loophole full hierarchy conformance.
9. Prove Nucleus direct-window composition excludes Surface packages.
10. Audit package direction and authority.

## Acceptance Criteria

- Surface bindings do not enter layout documents
- Surface resolution does not own display geometry
- returning preferred windows do not imply durable adoption
- native failure cannot fabricate committed Surface state
- hidden windows reveal only after placement and page readiness
- shutdown returns Surface flush evidence
- Loophole fixture resolves window to Surface to container
- Nucleus fixture resolves window to container with no Surface dependency
- no consumer URL, title, capability, or presence policy enters Longhorn state

## Evidence Required

- dependency and authority report
- mock full-hierarchy host fixture
- missing/returning-window table
- partial apply and shutdown receipts
- no-Surface build proof
- Rust 1.85 and full Effigy QA

## Stop Conditions

- Surface composition must bypass the existing window host
- window geometry enters Surface persistence
- Nucleus must link the optional package
- consumer creation or presence policy becomes a generic default
- Card 029 cleanup semantics are insufficient

## Next Task

Start Card 031.

## Outcome

Implemented `longhorn-surface-windowing` as the optional pure composition
layer. It maps resolved participating Surface hosts and existing placement
outcomes to plain desired-window inputs while retaining placement evidence.
It ignores direct-window outcomes, owns no geometry or native state, and
leaves URLs, titles, capabilities, presence, and creation policy with the
consumer.

The Tauri mock proof composes through the existing runtime-generic host. It
covers unsupported creation, injected creation, hidden placement,
page-readiness reveal, protected primary behavior, partial native failure,
retry, and ordered Surface-flush/window-shutdown receipts. Missing and
returning hosts and temporary display fallback do not mutate the Surface
document.

Loophole-shaped conformance resolves
`window -> Surface -> layout container -> region -> panel`.
`nucleus-no-surface-proof` compiles the direct
`window -> layout container -> region -> panel` shape without Surface
dependencies.

Cards 028-030 preserved Contract 011's host binding, revision, and persistence
authority. No transfer recompilation was needed. Card 031 is ready.
