# 012 Storage Profile Transition And Legacy Import

Status: complete
Owner: Tom
Roadmap: g01.002 batch 5
Governing refs: contracts 001, 003, 004, and 012; research memo 007
Auto-start next card: no

## Objective

Persist profile selection and migrate complete registered storage layouts
without stranding data, copying live databases, or deleting legacy authority.

## Scope

- fixed native bootstrap locator and schema
- explicit host bypass for tests, deployment, and portable launch
- source/target layout preview and conflict inventory
- declarative read-only legacy-root candidates
- transition plan bound to source, target, inventory, and current evidence
- deterministic dual-authority coordination
- ordinary-file staging, checksums, publication, and verification
- custom database adapter participation from card 010
- durable transition journal and crash recovery
- locator-last commit
- retained-source receipt and separate receipt-bound cleanup plan
- cache, log, runtime, secret, and unknown-file policies
- Loophole, Soundcheck, Nucleus, and Split-shell legacy discovery fixtures

## Public Behavior

A missing locator selects the compiled default. Invalid, future, or
unsupported locators enter recovery state and never choose another profile
silently.

Changing profile first produces a non-mutating plan. Execution copies and
verifies into the target, commits the locator last, and retains the source.
Cleanup is a later explicit operation against the transition receipt.

Cache is rebuilt by default. Runtime is never copied. Secrets stay in their
secure adapter. Live SQLite participates only through a native adapter.

## Out Of Scope

- consumer repository writes
- automatic legacy discovery outside registered candidates
- silent merge into a nonempty target
- cross-machine synchronization
- secure-store relocation
- settings-shell presentation

## Acceptance Criteria

- bootstrap resolution cannot recurse through selected config
- a locator with a different canonical application id fails closed
- corrupt and future locators expose recovery without fallback
- preview is side-effect free and binds both layouts plus current evidence
- overlapping roots and destination conflicts fail before mutation
- an injected failure before locator commit leaves the old layout authoritative
- an injected failure after locator commit recovers one verified authority
- source data remains until explicit receipt-bound cleanup
- cache, log, runtime, and secret policies are visible
- SQLite fixture uses the card-010 adapter, never ordinary file copy
- donor legacy candidates are discovered without write or delete
- unknown files are preserved and reported

## Stop Conditions

- profile selection must live only inside the selected root
- a transition can make two layouts authoritative
- failure recovery depends on best-effort guessing
- live database bytes must be copied directly
- cleanup can run without a matching verified receipt

## Next Task

Close `g01.002`. Compile `g01.003` display geometry, inventory, and pure
window-planning batches before implementation.
