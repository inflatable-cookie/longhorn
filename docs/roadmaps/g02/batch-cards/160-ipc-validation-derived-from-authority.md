# 160 IPC Validation Derived From The Authority

Status: ready
Owner: Tom
Roadmap: g02.011 batch 1
Governing refs: contracts 010 and 012; the P2-10 audit finding
Depends on: none
Auto-start next card: no

## Objective

Make the TypeScript IPC boundary's runtime validation derive from the Rust
authority that already defines it, instead of 5,330 hand-written lines that
nobody can currently describe.

## The Inventory — measured 2026-08-08

Thirteen packages carry a `compatibility` surface, applied at 187 call sites,
all inside `client.ts` / `connection.ts`. `svelte` and `poodle` are
downstream of the boundary and carry none; they are out of scope.

| package | LOC | key checks | ts bounds | rust `MAXIMUM_*` |
| --- | ---: | ---: | ---: | ---: |
| bridge | 821 | 20 | 14 | 8 |
| config | 858 | **0** | 3 | 0 |
| native-content | 824 | 59 | 3 | 0 |
| settings | 619 | **0** | 2 | 4 |
| operation | 476 | 42 | 1 | 3 |
| commands | 387 | **0** | 1 | 12 |
| history | 375 | **0** | 3 | 6 |
| notifications | 227 | 14 | 1 | 6 |
| transfer | 218 | **0** | 0 | 2 |
| surface-transfer | 147 | **0** | 0 | 0 |
| history-tree | 139 | **0** | 1 | 9 |
| surfaces | 129 | **0** | 0 | 2 |
| layout | 110 | **0** | 0 | 3 |
| **total** | **5,330** | 135 | 29 | 55 |

### Finding 1 — nine of thirteen packages never validate keys

Only bridge, native-content, operation and notifications reject an unknown
or missing field. The other nine check "is this an object" and stop:
`config`'s `record(value, path)` and `settings`' `record(value)` return the
value unexamined. Verified independently — zero occurrences of
`unknown_field`, `exactKeys`, or `Object.keys` across all nine.

That is 2,582 lines of validation that would accept a renamed field.

### Finding 2 — the bounds are magic numbers, and Rust names them

Rust declares 55 `MAXIMUM_*` / `HARD_MAXIMUM_*` constants across these
domains. TypeScript enforces 29 bounds, every one a hardcoded literal with
no link to its constant.

Bridge's six were checked against Rust by hand: `256/256/256/128/64/128`
against `MAXIMUM_REQUESTED_DOMAINS`, `MAXIMUM_CAPABILITY_DOMAINS`,
`MAXIMUM_AUTHORITY_DOMAINS`, `MAXIMUM_TRANSPORT_FEATURES`,
`MAXIMUM_DIAGNOSTICS`, `MAXIMUM_CAPABILITIES_PER_DOMAIN`. **All six
currently agree.** There is no live drift; there is also nothing preventing
it, because `ts-rs` discards the constants and no gate compares them.

Worse than drift is absence: `layout` (0 of 3), `surfaces` (0 of 2),
`transfer` (0 of 2), `commands` (1 of 12) and `history-tree` (1 of 9)
enforce almost none of the bounds their own crates declare.

### Finding 3 — the name is wrong, and that is a hazard

No package handles more than one protocol version. Every one asserts
equality with its single `_PROTOCOL_VERSION` and throws otherwise. There is
no compatibility logic in 5,330 lines named `compatibility`. A future author
adding real version negotiation will file it beside single-version
validators and inherit none of their assumptions.

### Finding 4 — the generated types are sufficient to derive from

Zero optional fields (`?:`) across every generated protocol file; 105
explicit `| null`. `ts-rs` emits `Option<T>` as always-present-and-nullable,
so there is no absent-versus-null ambiguity — the usual reason generated
validators end up weaker than hand-written ones. A derived validator can be
exactly as strict as `exactKeys` plus `nullableText` are today.

The generator already emits runtime values, not only types: every `_KINDS`
array and `_PROTOCOL_VERSION` constant the validators consume is generated
by `longhorn-bindings` (2,126 lines, 15 domain modules, golden fixtures, and
a `check:bindings` gate that already fails on drift). Emitting validators
extends an existing capability rather than adding one.

### Finding 5 — one thing is genuinely not derivable

`bridge/compatibility/negotiation.ts` carries a connection-state/reason
validity matrix (`ready: ["negotiationAccepted", "capabilityChanged"]`, and
so on). Both sides are plain string unions; the *pairing* rule exists in no
type. It stays hand-written and must be moved somewhere that says so.

## Scope

- an agreed target for what boundary validation checks, applied uniformly
- bound constants emitted from the Rust authority
- structural validators emitted per domain
- hand-written validators deleted package by package
- the state/reason matrix extracted as explicitly hand-owned

## Steps

1. Decide the target. Today's inconsistency means there is no "current
   behaviour" to preserve — nine packages would gain key validation and five
   would gain bounds. Record the target in contract 010 before generating
   anything.
2. Emit the `MAXIMUM_*` constants from the Rust authority into the generated
   protocol modules. Independently useful and shippable alone: it gives the
   29 existing magic numbers a source, and a diff gate.
3. Extract bridge's state/reason matrix into a hand-owned module with a
   comment saying why it is not generated.
4. Emit structural validators from the same authority as the types.
5. Migrate package by package, deleting the hand-written original in the
   same commit. A package carrying both generated and hand-written
   validators is worse than either.
6. Rename the surface. It validates; it does not reconcile versions.

## Acceptance Criteria

- every package rejects unknown and missing fields at the boundary
- every Rust `MAXIMUM_*` that bounds a wire-visible collection is enforced
  in TypeScript from a generated constant, never a literal
- no package carries both generated and hand-written validators
- the state/reason matrix is hand-owned and labelled
- `check:bindings` fails when a bound changes in Rust and not in TypeScript
- 187 call sites keep working; the 12 client modules are unchanged

## Evidence Required

- a re-run of this inventory showing uniform columns
- the bound-drift gate failing on a deliberate Rust-side change
- per-package deletion diffs

## Stop Conditions

- the generator cannot express a validator as strict as `exactKeys` for some
  domain, in which case that domain stays hand-written and is recorded as an
  exception rather than silently weakened

## Notes

Step 2 is worth doing even if the rest is deferred: it is small, it closes
the only finding here with a real drift mechanism, and it does not require
agreeing the target in step 1.

The inventory script is `/tmp/inventory.py` in the session that produced
this card; it is regex-based and undercounted twice before correction
(bridge's keyed `record` helper, and settings' `pub(crate) HARD_MAXIMUM_*`
naming). Re-derive rather than trust it.
