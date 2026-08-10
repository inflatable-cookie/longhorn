# 160 IPC Validation Derived From The Authority

Status: in progress — target decided; steps 2 and 3 landed on bridge
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

~~`bridge/compatibility/negotiation.ts` carries a connection-state/reason
validity matrix. Both sides are plain string unions; the *pairing* rule exists
in no type. It stays hand-written and must be moved somewhere that says so.~~

**Wrong, and corrected 2026-08-10 — see step 3.** The rule does exist in Rust.
It was in a `matches!` arm in `BridgeConnectionStatus::new`, which is not a
type, which is why `ts-rs` could not carry it — and why the conclusion "exists
in no type" slid into "cannot be derived". Those are different claims. It is
generated now.

## Step 2 Landed — 2026-08-08 (`37b49a0a`)

Bridge emits its eight `MAXIMUM_*` constants from the Rust authority, and
its negotiation validator consumes them instead of six literals.

**The drift gate was tested in both directions**, not assumed: changing
`MAXIMUM_DIAGNOSTICS` from 64 to 63 in Rust made `check:bindings` fail;
restoring and regenerating made it pass.

### Correction to Finding 2

The card assumed the remaining bounds were all hand-copied from constants
that exist. Measuring crate visibility says otherwise:

| package | `pub` | `pub(crate)` |
| --- | ---: | ---: |
| history-tree | 9 | 0 |
| history | 6 | 0 |
| notifications | 6 | 0 |
| operation | 3 | 0 |
| commands | 1 | 2 |
| settings | 0 | 4 |
| config, layout, native-content, surfaces, surface-transfer, transfer | 0 | 0 |

Three groups, not one:

- **history, history-tree, notifications, operation** — `pub`, follow the
  bridge pattern directly. 24 constants.
- **commands, settings** — behind `pub(crate)`, so a visibility decision
  comes first. Six constants.
- **config, layout, native-content, surfaces, surface-transfer, transfer**
  — declare no `MAXIMUM_*` at all. Their missing TypeScript bounds are a
  **Rust-side gap**, not a binding gap: there is no authority to derive
  from. Whether those domains should bound their collections is a separate
  question from generating validators, and belongs to whoever owns each.

## Step 2 Extended — 2026-08-08

Extending to the four `pub` packages found almost nothing to repoint, and a
better target instead.

**Correction to the inventory's "ts bounds" column.** It conflated
non-empty checks with maximums. Of the four packages, history has one real
maximum and the other three have none — their counted "bounds" are
`.length === 0` minimums. Across history, history-tree, notifications and
operation there is essentially **one** enforced maximum against 24 declared
Rust constants. So emitting those 24 would be preparation for step 4, not a
fix for existing drift, and it is deferred to the target decision.

**The cross-cutting bound was the real find.** `MAX_OPAQUE_ID_BYTES = 128`
lives in `longhorn-core/src/opaque_id.rs`, was **private**, and was
hand-copied into six sites across five packages: history, native-content,
config, and bridge (×3). It is now `pub`, emitted by four generators, and
consumed at all six sites. No literal remains.

**Retracted 2026-08-08 — there is no discrepancy.** This card previously
recorded a live validation hole: the constant bounds bytes, and three of the
six sites measure `value.length` (UTF-16 code units) rather than encoded
byte length. That reading was wrong. All six sites — including the three —
also enforce `/^[a-z0-9._:-]+$/`, and every character in that set is
single-byte ASCII, so for any value that passes the charset check the two
measurements are identical. Rust agrees: `opaque_id.rs` bounds `value.len()`
(bytes) and restricts to the same ASCII set.

The mismatch is therefore cosmetic. Bridge's `TextEncoder` is belt-and-braces
against a charset that cannot produce multi-byte input; the other three are
equivalent and cheaper. No behaviour change is warranted, and the step 1
target decision does not inherit this question.

The error came from reading the unit mismatch in isolation without checking
the adjacent charset guard — worth remembering, because the same shape
(a bound plus a charset restriction that makes the bound's unit moot)
recurs across these validators.

## Step 3 Landed — 2026-08-10

The state/reason matrix is generated, not extracted.

Card 160 planned to move it into a hand-owned module with a comment saying why
it could not be generated. Reading `longhorn-bridge` first showed the premise
was false: `BridgeConnectionStatus::new` carried the same eleven arms in a
`matches!`, and the two copies **agreed exactly** — by maintenance rather than
by construction.

So Rust declares the table instead of pattern-matching it.
`BridgeConnectionStatus::ADMITTED_REASONS` is a `pub const` that `new()`
consults, `longhorn-bindings` emits as
`BRIDGE_ADMITTED_CONNECTION_REASONS`, and the TypeScript validator reads. The
eleven-arm literal is gone.

Wire names come from serde rather than variant identifiers, so a `rename_all`
change moves both sides together rather than silently desynchronising them.

**The drift gate was tested in both directions**, as step 2's was: adding
`HostFailure` to `Offline` in Rust made `check:bindings` fail and name the
artifact; restoring made it pass. All 23 bridge tests pass against the declared
table, so `new()`'s behaviour is unchanged.

### What this says about the inventory's findings

Finding 5 was the only one asserting something could not be done, and it was
the one that turned out to be wrong. The mistake was reading "exists in no
type" — true, `ts-rs` carries types — and concluding "cannot be derived", which
does not follow. A generator can emit anything Rust can declare; the question
is whether Rust declares it, not whether it is a type.

Worth carrying into step 4: the remaining "not derivable" judgements in this
card deserve the same check before they are trusted.

## Step 1 Decided — 2026-08-10

The target is in contract 010. The decision was less open than this card
assumed, because the measurement that settles it had not been taken.

**Rust declares `deny_unknown_fields` on 332 types** — config 64, bridge 29,
settings 26, operation 21, notifications 16. Nine TypeScript packages accept
unknown fields. The boundary has therefore been asymmetric all along: a payload
TypeScript waves through is one Rust refuses. That is not a posture anyone
chose; it is the shape of who wrote which validator.

Matching the authority is the only option that leaves the boundary describable
in one sentence, and "derived from the authority" is what this card is called.

The inventory's other load-bearing findings were re-measured before betting a
contract amendment on them, since Finding 5 had already proved wrong:

- **Finding 1 holds.** Four packages check keys — bridge, native-content,
  operation, notifications. Nine do not.
- **Finding 4 holds.** Zero optional fields across 4,682 lines of generated
  protocol, 88 explicit `| null`. The count has drifted from the card's 105 as
  protocols changed; the claim it supports has not.

The cost is stated in the contract rather than left implicit: nine packages
become stricter, and a consumer sending an undeclared field starts failing.
Every consumer is on a `file:` install, so it surfaces during a coordinated
change rather than in the field.

## Scope

- an agreed target for what boundary validation checks, applied uniformly
- bound constants emitted from the Rust authority
- structural validators emitted per domain
- hand-written validators deleted package by package
- the state/reason matrix extracted as explicitly hand-owned

## Steps

1. ~~Decide the target.~~ Decided 2026-08-10 and recorded in contract 010's
   **Boundary Validation Target**: the boundary matches the Rust authority's
   strictness and derives it. Unknown fields rejected, missing fields
   rejected, every bound from a named constant.
2. Emit the `MAXIMUM_*` constants from the Rust authority into the generated
   protocol modules. Independently useful and shippable alone: it gives the
   29 existing magic numbers a source, and a diff gate.
3. ~~Extract bridge's state/reason matrix into a hand-owned module.~~ Landed
   2026-08-10 as generation instead — the premise that it could not be derived
   was wrong.
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
- [x] the state/reason matrix has one source — generated from
  `BridgeConnectionStatus::ADMITTED_REASONS`, not hand-owned as planned
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
