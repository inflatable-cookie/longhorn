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
| history | 375 | 15 | 3 | 6 |
| notifications | 227 | 14 | 1 | 6 |
| transfer | 218 | **0** | 0 | 2 |
| surface-transfer | 147 | **0** | 0 | 0 |
| history-tree | 139 | 14 | 1 | 9 |
| surfaces | 129 | **0** | 0 | 2 |
| layout | 110 | **0** | 0 | 3 |
| **total** | **5,330** | 164 | 29 | 55 |

### Finding 1 — seven of thirteen packages never validate keys

*Corrected 2026-08-10. This finding first said nine, and named `history` and
`history-tree` among them. Both were already strict.*

Bridge, native-content, operation and notifications reject an unknown or
missing field — and so do `history` (15 call sites) and `history-tree` (14).
The other seven check "is this an object" and stop: `config`'s
`record(value, path)` and `settings`' `record(value)` return the value
unexamined.

That is 2,068 lines of validation that would accept a renamed field.

**How the count was wrong, because the mechanism matters.** The verification
was one shell loop over the thirteen packages, resolving each package's
compatibility files with a glob that included `compatibility/*.ts`. Seven
packages have no such subdirectory. Under `zsh` an unmatched glob is a fatal
error for the whole command, so the file list came back empty and every
package counted zero. The four that were reported as strict were counted from
a different, earlier command.

A loop that reports zero for every input is not evidence of zero; it is
evidence the loop did not run. The same failure reproduced exactly when the
count was re-taken, which is how it was found.

What `history` and `history-tree` actually needed was therefore not
strictness but *provenance*: their key lists were hand-written literals, and
nothing tied them to the Rust structs they mirror.

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

## Step 4 Design Notes — 2026-08-10

Read `surfaces` (129 lines, the smallest of the nine) before writing a
generator, and the shape of the work is narrower than "emit structural
validators" suggests.

### What the nine actually lack is a field list, not a validator

`surfaces` already has the right skeleton: an incompatibility error type, a
protocol-version assert, kind checks against generated `_KINDS` arrays, and a
`record()` helper. What `record()` does not do is take the allowed keys —
bridge's takes `(value, required, optional)` and rejects anything else, and
that difference is the whole of Finding 1.

So the minimum that satisfies the contract target is **emit the field lists**,
not emit whole parsers:

```ts
export const SURFACE_FIELDS = {
  SurfaceSnapshot: ["protocol_version", ...],
  ...
} as const;
```

Derivable from the same `ts-rs` declarations the generator already renders,
and it makes every existing `record()` call strict by passing one argument.
That is a much smaller generator than a parser-combinator emitter, and it
closes the asymmetry the contract names.

Whether to go further — generating the parsers themselves, so the hand-written
skeletons go too — is a second decision. The field-list step is worth taking
first because it is independently shippable and because step 5's
package-by-package deletion is safer once every package is already strict.

### Two things noticed in passing

`surfaces` throws `"unknown_response_status"` when `record()` is handed a
non-object, because its incompatibility union has no `invalid_object` code.
The error is wrong and cheap to fix; every one of the nine should be checked
for the same reuse.

The order matters for step 5. A package made strict by field lists still
carries its hand-written skeleton, and the card's own rule is that carrying
both generated and hand-written validators is worse than either. The field
list is a *change to* the hand-written validator, not a second one beside it,
so it does not trip that rule — worth stating, because it looks like it might.

### Landed on surfaces — 2026-08-10

`generation::field_map` emits `<DOMAIN>_FIELDS` from the `ts-rs`
declarations, and `surfaces` is the first package migrated. Its `record()`
takes the allowed keys and rejects unknown fields, missing fields, and
non-objects. Four tests, and the drift gate proved in both directions: adding
a field to `SurfaceChangedEvent` in Rust failed `check:bindings`; removing it
passed.

**The parser needed two corrections, both found by running it.** Its first
output put prose in field names, because `ts-rs` renders Rust doc comments as
`/** .. */` between fields and those contain colons and commas. And it let
tagged unions through, because their braces balance — a naive balance check
sees `{ .. } | { .. }` as well-formed. A union is now rejected on containing
any brace inside its body, and the skipped names are printed rather than
dropped: `EmptyWindowPolicy`, `SurfaceMutationCommand`,
`SurfaceMutationOutcome`, `SurfaceMutationRejectionCode`,
`SurfaceMutationResponse`.

**Tagged unions keep the lenient path deliberately.** Their allowed keys
depend on the discriminant, so one flat list is wrong for them; `record()`
called without a list keeps its old shape-only behaviour, and the code says
so. Handling them properly is a per-variant field map and is not attempted
here.

**The wrong error code is fixed as predicted.** `surfaces` threw
`unknown_response_status` for a non-object because its union had no code for
one. It now has `invalid_object`, `unknown_field` and `missing_field`.

### transfer and surface-transfer — 2026-08-10

Both had `surfaces`' exact shape, and both took the same two edits. Strictness
tests on each, and `TransferClientSnapshot`'s list is asserted against the
fixture's own keys so the two cannot drift apart quietly.

**Typing a call site by name-matching is how this goes wrong.**
`assertCompatibleSurfaceTransferTarget` got `SurfaceTransferCommand`'s list
because the names looked adjacent; it validates a `SurfaceTransferTarget`,
which is a tagged union the generator had already refused to emit a list for.
The result rejected `kind` — the discriminant — and two existing tests failed
immediately.

That is the system working twice over: the generator declined to guess, and
the strictness caught the guess made on its behalf. The rule for the remaining
packages is to read what the function asserts, not what the constant is
called.

The strictness also found a stale test fixture: `transfer`'s snapshot fixture
carried `windows` and `zones`, which the type has not had. It never failed,
because nothing checked.

### config — 2026-08-10

The package the inventory named first: 858 lines of validation, zero key
checks. Forty-seven object types now have field lists, emitted from both
`base` and `restore` declaration sets into one `CONFIG_FIELDS` map — the
validators import from `protocol.ts`, which re-exports both, so splitting the
map would only make callers guess which half a type lives in.

`record(value, path, allowed?)` keeps `path` ahead of the list because every
existing caller passes one and it is what makes a rejection locatable in a
nested document. A rejection now reports `$.field` rather than the parent.

**Four validators typed, and the rest deliberately not yet.**
`ConfigOperationsSnapshot`, `RestoreInspectionProjection`,
`RestorePlanProjection` and `RestoreExecutionReceiptProjection` are the ones
whose asserted type is stated in their own signature. The remaining call sites
take a `path` and validate a nested fragment whose type has to be traced;
after the `SurfaceTransferTarget` mistake, tracing them is the work rather
than a formality.

So `config` is partially migrated, and that is stated rather than implied: its
top-level entry points are strict and its nested fragments are not.

### settings — 2026-08-10

619 lines of validation and zero key checks before this. Twenty-seven object
types mapped; `SettingsScopeSnapshot` and `SettingsLoadCommand` typed, and the
code union gained `unknown_field` and `missing_field`.

Same partial shape as `config`, and stated the same way: the entry points
whose asserted type is in their own signature are strict, and the nested
fragments are not.

**One mistake, and it is the second of its kind.** The import guard checked
whether `SETTINGS_FIELDS` appeared in the file *after* the replacement had
already inserted it, so it decided the import was present and skipped it. Five
tests failed on an undefined name. The same ordering error as the earlier
`observe_into_cache` bug — a condition evaluated against state the preceding
statement had already changed.

### commands, history, history-tree — 2026-08-10

Three packages in one pass, because measuring them properly showed they were
three different jobs rather than one repeated three times.

`commands` was the real gap: 387 lines, twelve Rust `MAXIMUM_*` constants,
and no key checking anywhere. Its `object(value, path)` gained the same
optional `allowed` as the others, and eight validators are now typed — seven
directly plus `CommandKeymapPreview` through the shared `baseRequest`, which
also gained the parameter. `keymapPatch` validates exactly one type, so it
carries `CommandKeymapPatch` unconditionally.

`history` and `history-tree` were already strict. What they got is
provenance: 11 hand-written key lists each replaced by the generated map, so
a renamed Rust field now moves the TypeScript with it. Both keep their
literals at the tagged unions, whose allowed keys depend on a discriminant.

**A near-miss worth recording.** The replacement was scripted — match a
literal array argument, look up its exact key set, swap in the constant. In
`history-tree` one call site was `exact(root, "$", [...common, ...extra,
...])`, where `extra` distinguishes the two page commands. The script matched
the bracketed text, extracted only the quoted literals, found that set equal
to `ForkBranchPageCommand`, and rewrote the call with that fixed list —
silently discarding the parameter that made the function serve both types.
`ForkPathPageCommand` would then have been rejected for carrying `target`.

No test caught it. It was found by reading the diff, and the fix was to make
the helper take the field list rather than a difference. A regression test now
holds the two commands distinct. The lesson is narrow and real: a
transformation that pattern-matches source text will match things that look
like its target and are not, and the arity of a helper is exactly the sort of
thing it cannot see.

### layout has no boundary to validate

`layout` was on the list as a package with zero key checks. It has zero key
checks because it has no client: the directory is `compatibility.ts`,
`ratio.ts`, `visibility.ts` and an index, and nothing in it receives an IPC
payload. Its four exports check a protocol version and three tagged-union
discriminants — none of which a field map covers.

Giving it a `record` would be building a boundary the package does not own.
It is removed from step 4's scope rather than left open.

## Step 2 Landed — 2026-08-10

Every wire-visible bound now comes from a named Rust constant.

Five literals were still hardcoded in TypeScript, and the interesting part is
that two of them had a generated constant sitting unused in the same package:
`BRIDGE_MAXIMUM_DIAGNOSTIC_MESSAGE_BYTES` and
`BRIDGE_MAXIMUM_FAILURE_MESSAGE_BYTES` were both emitted, both correct, and
both ignored in favour of `4096` written out twice. Emitting a constant is not
the same as using it, and nothing had been checking the difference.

| literal | now | source |
| --- | --- | --- |
| `4096` ×2 (bridge) | `BRIDGE_MAXIMUM_{DIAGNOSTIC,FAILURE}_MESSAGE_BYTES` | already emitted, unused |
| `65_536` (bridge) | `BRIDGE_MAXIMUM_DEDUPLICATION_ENTRIES` | newly emitted |
| `4_096` (history) | `HISTORY_MAXIMUM_PROJECTION_PAGE_SIZE` | newly emitted |
| `64` (commands) | `COMMAND_MAXIMUM_PHYSICAL_CODE_BYTES` | newly emitted |
| `16_384` (settings) | `SETTINGS_HARD_MAXIMUM_TEXT_BYTES` | newly emitted |
| `1_048_576` (settings) | `SETTINGS_HARD_MAXIMUM_OPAQUE_VALUE_BYTES` | newly emitted |

Five Rust constants had to widen visibility to be emitted — four
`SettingsLimits::HARD_MAXIMUM_*` from `pub(crate)`, and
`MAXIMUM_PHYSICAL_CODE_BYTES` from private. That is not a concession: each was
already reflected in TypeScript, so each was public API in fact before it was
public in visibility.

`settings`' `HARD_MAXIMUM_OPAQUE_VALUE_BYTES` keeps its name, so its twenty
call sites do not churn; only its value moved to the generated source. It
needed an explicit `: number`, because the generated constants are `as const`
and the literal type `1048576` is not assignable to a `number` parameter. The
type checker caught that immediately.

### The drift gate, demonstrated

The card asks for the gate failing on a deliberate Rust-side change, so:
`HARD_MAXIMUM_TEXT_BYTES` changed from `16_384` to `8_192` in Rust only.

```
$ effigy check:bindings
longhorn-bindings: generated settings artifacts drifted:
  packages/longhorn/src/settings/generated/protocol.ts; run `effigy generate:settings`
exit 1
```

Named the file and named the fix. Reverted; the gate returned to zero.

One thing this does *not* catch: a bound that exists in Rust and is enforced
nowhere in TypeScript. The gate compares generated output against generated
output, so a constant nothing imports drifts silently — which is exactly how
bridge's two message-byte constants sat unused. Emission is gated; *use* is
not.

### config's nested fragments — 2026-08-10

34 more call sites typed across four files. `config` is now strict everywhere
it validates a plain object; what stays lenient is exactly the tagged unions,
each of which discriminates on a `status`, `kind`, `state` or `source` field
one line later.

Two shared helpers took the field list as a parameter rather than being
split: `baseCommand`, called by ten command validators, and
`generationConfirmationCommand`, called by four. This is the same shape as
history-tree's `commandBase` — done deliberately this time.

**The passing test suite was not evidence, and measuring that mattered.**
Applying 18 mappings and seeing config's nine tests still pass says nothing
about a mapping on a path the fixture never reaches. So each mapping was
probed: inject a sentinel key into that type's field list, re-run, and see
whether anything fails. A mapping whose tests still pass with a deliberately
impossible field list is unverified.

The first probe came back **17 of 18**, and the unproven one was a bug in the
edit rather than a gap in the fixture. The script appended the field-list
argument before the line's last `);` — which on

```ts
identity(record(inspection.identity, `${path}.identity`), `${path}.identity`);
```

is `identity(...)`'s closing paren, not `record(...)`'s. It passed a third
argument to a two-parameter function. `bun test` does not type-check, so the
suite stayed green; only the probe found it. Fixed, re-probed, 18 of 18. The
later batch of 16 probed clean on the first pass.

That is the second time today a scripted source transformation matched
something that looked like its target and was not. The pattern is now
explicit: **script the edit, but verify by execution, not by re-reading the
script.**

### Remaining

`settings`' nested fragments.

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
2. ~~Emit the `MAXIMUM_*` constants from the Rust authority into the generated
   protocol modules.~~ Landed 2026-08-10. Six literals replaced across four
   packages; two of them already had an emitted constant that nothing used.
3. ~~Extract bridge's state/reason matrix into a hand-owned module.~~ Landed
   2026-08-10 as generation instead — the premise that it could not be derived
   was wrong.
4. Emit structural validators from the same authority as the types.
5. Migrate package by package, deleting the hand-written original in the
   same commit. A package carrying both generated and hand-written
   validators is worse than either.
6. Rename the surface. It validates; it does not reconcile versions.

## Acceptance Criteria

- [x] every package rejects unknown and missing fields at the boundary — at
  the top level; nested fragments in `config` and `settings` remain
- [x] every Rust `MAXIMUM_*` that bounds a wire-visible collection is enforced
  in TypeScript from a generated constant, never a literal
- no package carries both generated and hand-written validators
- [x] the state/reason matrix has one source — generated from
  `BridgeConnectionStatus::ADMITTED_REASONS`, not hand-owned as planned
- [x] `check:bindings` fails when a bound changes in Rust and not in TypeScript
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
