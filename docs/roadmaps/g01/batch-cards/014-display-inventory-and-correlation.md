# 014 Display Inventory And Correlation

Status: complete
Owner: Tom
Roadmap: g01.003 batch 2
Governing refs: contracts 001, 003, 009, and 012; research memo 003
Auto-start next card: no

## Objective

Add a pure `longhorn-display` package for persistent known displays, current
observations, confidence-bearing correlation, ambiguity, availability, labels,
and arrangement signatures.

## Scope

- persistent known-display records keyed by Longhorn `DisplayId`
- current observed-display records with optional adapter and hardware keys
- available, missing, and unresolved observation states
- machine labels, user labels, main and built-in hints
- remembered adapter keys without promoting them to canonical identity
- strongest-key, remembered-key, exact-geometry, and unique-weak correlation
- explicit confidence and evidence on every match
- unresolved ambiguous weak matches
- injected identity allocation for genuinely new known displays
- explicit forget operation
- deterministic arrangement signature over available canonical displays
- Loophole, Nucleus, and Soundcheck-shaped fixtures

## Public Behavior

`DisplayId` is machine-local Longhorn identity. Platform ids, hardware keys,
names, geometry, scale, and adapter ids remain evidence. No evidence field
becomes canonical identity by constructor or fallback.

Reconciliation evaluates strong platform/hardware evidence, remembered adapter
evidence, exact geometry plus scale, then a unique weak fingerprint. Weak
evidence binds only when both sides are unique. Ambiguity returns candidates
and confidence without mutating remembered evidence.

Known displays remain present as missing records when not observed. Labels and
placement-facing identity survive rearrangement and absence. A new identity is
allocated only through an injected allocator after correlation leaves one
unmatched observation. Enumeration order cannot change matches, ambiguity, or
the arrangement signature.

## Out Of Scope

- direct Tauri monitor calls
- non-macOS strong-key acquisition
- ambiguity UI or prompts
- user synchronization across machines
- window fallback, placement, or live apply plans
- forgetting displays automatically
- TypeScript, Svelte, Poodle, Surface, or donor-repository writes

## Steps

1. Add `longhorn-display` with only `longhorn-core` plus narrow serialization
   and test dependencies.
2. Define strict known, observed, evidence, confidence, match, ambiguity, and
   availability types.
3. Implement order-independent one-to-one strong and remembered-key matching.
4. Implement exact geometry/scale and unique weak-fingerprint matching.
5. Return ambiguity without applying a match or overwriting remembered keys.
6. Reconcile new, available, and missing known records through injected ids.
7. Build a versioned deterministic arrangement signature from sorted available
   canonical ids, full bounds, work areas, scale, and main status.
8. Port donor-shaped fixtures and run complete validation. Stop before window
   placement.

## Acceptance Criteria

- a new display receives one injected Longhorn id and retains it
- platform, hardware, adapter, name, and geometry observations never become ids
- strong evidence wins over remembered, exact geometry, and weak evidence
- remembered adapter evidence wins over geometry-only evidence
- ambiguous weak candidates remain unresolved and do not mutate known records
- exact duplicate weak fingerprints do not bind by enumeration order
- missing displays retain labels, remembered evidence, and identity
- user label wins over machine label without erasing either source
- explicit forget removes only the named known display
- arrangement signature ignores enumeration order
- arrangement signature changes when canonical availability, bounds, work
  area, scale, or main status changes
- no available display produces the stable empty arrangement
- package graph has no Tauri, windowing, config, Svelte, Poodle, or Surface edge

## Evidence Required

- Loophole exact, geometry-only, rearranged, remembered-key, and ambiguity cases
- macOS strong-key fixture without a Core Graphics runtime dependency
- Nucleus synthetic name/position/size key classified as weak evidence
- missing/reappearing display fixture preserving Longhorn identity and label
- duplicate-monitor ambiguity fixture with permuted input order
- arrangement-signature permutation tests
- injected allocator call-count and id-retention tests
- Rust 1.85 workspace check
- `effigy doctor`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`

## Stop Conditions

- a host observation must become canonical identity
- ambiguous weak evidence would bind automatically
- correlation depends on adapter enumeration order
- known missing records must be deleted or relabelled
- random or platform id generation becomes hidden inside the pure resolver
- non-macOS host probing or ambiguity UI enters the package
- Tauri, config, windowing, Svelte, Poodle, Surface, or product types are required

## Next Task

Cards 015-017 and `g01.003` are complete. Card 018 is the sole ready
`g01.004` lane.
