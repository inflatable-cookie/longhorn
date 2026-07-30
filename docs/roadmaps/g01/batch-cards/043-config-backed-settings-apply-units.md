# 043 Config-backed Settings Apply Units

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.008 batch 1
Governing refs: contracts 004, 005, and 010; research memo 012
Depends on: Card 042
Auto-start next card: no

## Objective

Bind pure settings apply units to fresh coordinated configuration mutation with
exact stale-token, policy, reset, durability, and activation behavior.

## Scope

- `longhorn-settings-config` crate
- narrow checked-mutation seam in `longhorn-config` if required
- typed consumer projector, patch, validation, reset, and activation adapters
- one registered domain per built-in apply unit
- configured/default/effective/policy projection
- forced, constrained, read-only, hidden, and unsupported policy outcomes
- immediate and staged command execution
- authority-token conflict and resnapshot
- exact mutation and activation receipts
- explicit consumer transaction-authority seam for broader units

## Public Behavior

The adapter compares the host-issued token against fresh authority while the
existing coordinator is held. A stale, invalid, policy-blocked, or recovery
state publishes nothing.

One config-backed apply unit mutates one domain atomically. A consumer may
provide another transaction authority, but the built-in adapter and receipts
never imply cross-domain atomicity.

Reset removes only the registered user override. Activation is computed after
successful persistence and reported separately.

## Out Of Scope

- generic product schema or field renderer
- policy file format
- remote or multi-machine authority
- hidden retry or renderer-side merge
- TypeScript, Tauri, Svelte, or Poodle

## Steps

1. Prove whether current config mutation can compare source evidence under its
   existing coordinator.
2. Add the smallest checked mutation seam if comparison is not currently
   atomic.
3. Define typed settings projection and mutation adapter traits.
4. Bind one apply unit to one registered domain and descriptor.
5. Project default, configured, effective, policy, editability, and recovery.
6. Reject stale tokens, policy violations, invalid intent, and unavailable
   authority before publication.
7. Apply patch and reset through coordinated config publication.
8. Return exact durability, new authority token, snapshot, and activation.
9. Add concurrency, failure injection, policy, reset, and multi-unit fixtures.

## Acceptance Criteria

- token comparison and mutation occur under one coordinator authority
- an intervening writer causes conflict without publication
- policy-forced and read-only fields cannot mutate
- constrained values validate at host authority
- configured value may differ visibly from policy-effective value
- immediate and staged commands use the same authoritative apply path
- invalid staged apply leaves current bytes unchanged
- reset cannot alter policy, defaults, another domain, or secret authority
- success returns fresh snapshot, exact durability, and activation separately
- built-in units cannot span two config domains
- broader consumer authority is explicit and separately receipted

## Evidence Required

- checked-mutation authority proof
- conflict, policy, validation, recovery, and publication failure matrix
- reset scope fixtures
- activation matrix over immediate and staged timing
- two-process/intervening-writer fixture
- dependency and atomicity audit
- focused Rust and Effigy QA

## Stop Conditions

- token comparison requires a second lock acquisition
- current configuration API cannot expose exact source evidence safely
- policy precedence needs product semantics inside Longhorn
- failure atomicity would require a generic multi-domain claim
- a remote authority is required

## Next Task

Card 044 is ready but not started. Generate the checked protocol and add the
narrow Tauri host over injected settings authorities.

## Result

`longhorn-config` now exposes one narrow `mutate_checked` path. It acquires the
existing coordinator once, recovers, rereads fresh state, exposes exact source
bytes to a veto-capable patch, validates, and publishes only accepted changed
bytes.

`longhorn-settings-config` binds one sealed apply unit to one ordinary writable
config domain. Consumer adapters retain typed projection, intent, constraint
validation, patch, reset, policy, and activation semantics. The binding checks
registry/page/scope authority, rejects stale tokens and non-editable targets
before patch, returns fresh snapshots and exact config durability, and invokes
activation only after the config call succeeds.

Fourteen contract tests cover immediate and staged timing, unchanged writes,
forced and constrained policy, read-only/hidden/unsupported fields, invalid
intent, scoped reset, corrupt recovery, publication failure, source evidence,
and a real helper-process intervening writer. The consumer transaction trait
keeps broader atomicity explicit and separately receipted.

Evidence:
`../../../logs/2026-07/29-config-backed-settings-apply-units.md`.
