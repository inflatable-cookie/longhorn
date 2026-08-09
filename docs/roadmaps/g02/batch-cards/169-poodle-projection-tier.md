# 169 Poodle Projection Tier

Status: blocked — no mechanism exists for Longhorn to depend on Poodle's Rust crates
Owner: Tom
Roadmap: g02.012 follow-up
Governing refs: contract 020; contract 013; research memo 021
Depends on: Card 163
Auto-start next card: no

## Objective

Give a GPUI application something to draw. `longhorn-gpui-windowing` opens and
places a window; nothing renders into it.

## Why this exists

Memo 021 named this and no card ever scheduled it:

> A GPUI host therefore needs a projection tier, not a parallel of
> `packages/*`.

Card 163 finished the host boundary and confirmed the memo's structural
claim: a GPUI application needs **one** host adapter where Tauri has eleven,
because nine of the Tauri crates are IPC handler assembly a GPUI application
never composes, and the rest are value boundaries that work unchanged.

What it does not have is the layer above. `longhorn-poodle-svelte` projects
Longhorn's domains — commands, config, history, notifications, operation,
settings — into Svelte components against Poodle. Its Rust sibling does not
exist, so today a GPUI product can place a window and must then hand-write
every projection itself, which is precisely the duplication a framework
exists to prevent.

## Structural questions — answered 2026-08-09, before code

Both questions this card said to settle first are settled, and one changed the
card's premise.

### The name is `longhorn-poodle`, not `longhorn-poodle-gpui`

Card 164 asserted the Rust sibling "will be `longhorn-poodle-gpui`". Measuring
Poodle says otherwise. `poodle-specs` — Poodle's shared Rust contract layer —
already has **two** renderers over it:

| Adapter | Lines | Maps specs to |
| --- | ---: | --- |
| `poodle-gpui` | 4,408 | GPUI elements |
| `poodle-jetstream` | 5,025 | Jetstream `UiTree` nodes |

Both describe themselves identically: "maps shared contract specs and resolved
styles to <renderer> nodes". So a Longhorn projection that emits **specs** is
renderer-neutral by construction, and works for Jetstream and anything after
it without change.

Naming it `-gpui` would bake one renderer into something that is not
renderer-specific, and would invite a second copy the first time Jetstream
needs the same projection. `longhorn-poodle` is the sibling of
`longhorn-poodle-svelte`, where the suffix names the *framework binding* — and
Card 164 freed exactly that name when it renamed the TypeScript package.

### The `gpui` dependency does not arise

This card feared the dependency it inherited from Card 163's compromise. It
does not apply. Poodle's own adapter is deliberately `gpui`-free — it emits
`GpuiStyle`, `GpuiColor`, `GpuiEdges`: GPUI-*shaped* plain data, with a
comment saying so outright — and only `packages/gpui/preview` builds real
elements.

So the chain is:

```text
Longhorn domains -> longhorn-poodle -> poodle-specs
                                          |-> poodle-gpui      -> gpui elements
                                          `-> poodle-jetstream -> UiTree nodes
```

`longhorn-poodle` depends on `poodle-specs` and nothing heavy. It is an
ordinary workspace crate, inside `effigy qa`, at no cost to any lane. The
"heavyweight host SDKs have no in-gate home" papercut is untouched by this
card — it remains a real problem for `prototypes/gpui-windowing` and is not
one this card has to solve.

This is the same shape twice over: `longhorn-gpui-windowing` takes no `gpui`
and pushes the binding to an excluded prototype; `poodle-gpui` takes no `gpui`
and pushes it to a preview. Two repositories reached it independently.

## Blocked — 2026-08-09: there is no Rust route from Longhorn to Poodle

The name and the dependency shape are settled, and then the card stops. It
cannot start, and the reason is mechanical rather than hard.

`longhorn-poodle` must depend on `poodle-specs`. There is no sanctioned way
for it to do so.

| Route | State |
| --- | --- |
| crates.io | `poodle-specs` is not published |
| private registry | none configured for either repository |
| git dependency | not used anywhere in either repository |
| cross-repo path | breaks CI, which checks out Longhorn alone |
| pinned artifacts | **exists for npm only** |

Longhorn already consumes Poodle, and does it well: pinned tarballs from
`~/Dev/projects/poodle/.artifacts/`, each with a recorded SHA-256, and a set
id that is a membership hash over `name:sha256` pairs. Contract 012 records
the pins; `scripts/poodle-evidence.ts` derives them from the root manifest so
they cannot rot. That is a considered mechanism with an integrity claim.

It is **entirely npm**. `grep -rn poodle --include=Cargo.toml` across Longhorn
returns nothing, and Poodle's `.artifacts/` holds only `svelte-pack-install-*`
sets. Poodle's Rust crates are path dependencies inside Poodle and reach no
further.

### Why not just add a path dependency

Because it would be the first thing in either repository to assume a fixed
relative checkout, it would fail in CI, and it would carry no integrity claim
at all — against a contract whose whole cross-repo model is pinned hashes.
A one-line `path = "../poodle/…"` would work on this machine and nowhere else.

### The options, for a decision

1. **Publish Poodle's Rust contract crates to crates.io.** Simplest, and
   already the direction of travel: g02.014 is "Poodle and Longhorn on public
   npm". Publishing the Rust contract layer publicly is the consistent
   parallel. Cost: `poodle-specs` becomes public API with a versioning
   commitment, which it does not have today.
2. **Extend the artifact-pinning model to Rust.** Most consistent with
   contract 012 as written — vendored crate sources with recorded hashes, the
   same integrity claim the npm side already makes. Cost: building a
   mechanism that does not exist, for one consumer.
3. **Wait.** The projection tier is not on the critical path for anything
   shipping. Cost: GPUI applications stay unable to draw.

Option 1 is the recommendation, on the grounds that the parallel already
exists and the alternative is inventing distribution machinery for a single
dependency edge.

**This is a cross-repository distribution decision touching contract 012, so
it is recorded rather than taken.**

## Scope

- `longhorn-poodle`: projections from Longhorn domains to Poodle specs,
  renderer-neutral
- the domains Card 163's first target actually needs, and no more
- a parity statement against `longhorn-poodle-svelte`, per domain

## Explicit non-goals

- Porting `packages/*`. The IPC tier does not exist here; nothing serializes.
  Memo 021 measured roughly 18,000 of ~24,000 TypeScript lines as boundary
  machinery that simply disappears.
- Forking Poodle's primitives. `poodle-gpui` renders; this projects onto it.
  Contract 013's rule holds — Longhorn ships the projection that drives the
  kit, and the kit stays generic.
- Feature parity with the Svelte tier. Parity is *stated per domain*, not
  assumed.

## Steps

1. ~~Decide the name~~ — done, see above: `longhorn-poodle`.
2. Pick the domains from the first GPUI target's actual needs — memo 021 says
   config, settings, operations, notifications, licence and update. Layout,
   surfaces and transfer are **not** in that list and should not be built
   speculatively.
3. Build one domain end to end first, and use it to find where the Svelte
   projections encode a webview assumption. Card 163's lesson is that the
   second implementation is what exposes the first one's hidden shape, and
   the projections have had exactly one implementation so far.
4. State parity per domain in a table: projected, deliberately absent, or not
   yet. "Not yet" and "will not" are different claims and must not blur.
5. Take the same dependency-direction discipline: this crate depends on
   `poodle-specs`, and Poodle keeps no reference to Longhorn.

## Acceptance Criteria

- [x] the crate name is a recorded decision, not an inherited assumption
- one domain is projected end to end and renders through at least one adapter
- parity is stated per domain, with "deliberately absent" distinguished from
  "not yet"
- no Poodle primitive is forked
- Poodle contains no reference to Longhorn
- [x] the `gpui` dependency's home is decided: it does not arise, because the
  projection emits specs

## Evidence Required

- the projection for one domain, and a running window that draws it
- the parity table
- a statement of which renderers the projection was checked against

## Stop Conditions

- a Longhorn domain cannot be projected without changing a Poodle primitive,
  in which case the gap is raised in Poodle rather than forked here
- the first domain reveals that the Svelte projections encode webview
  assumptions deeply enough to need a contract 013 amendment, in which case
  that amendment is compiled before more domains are built

## Notes

Sequencing: this is the largest remaining GPUI piece and the only one
blocking a real GPUI product. Contract 020's unproven claims — cross-window
transfer, lifecycle teardown under load — stay unproven until an application
exists to exercise them, and this card is what makes such an application
possible.

## Next Task

Whatever the first domain reveals.
