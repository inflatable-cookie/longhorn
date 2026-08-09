# 169 Poodle Projection Tier

Status: in progress — first domain projected
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

## Dependency mechanism — corrected 2026-08-09

This card was briefly recorded as blocked on the grounds that Longhorn had no
sanctioned way to depend on Poodle's Rust crates. That was wrong, and the
error was mine: I treated a cross-repo path dependency as disqualifying
without checking that Longhorn already uses one.

It does. `package.json` carries
`"@inflatable-cookie/poodle-core": "file:../poodle/.artifacts/…"`. A sibling
path reference is the established temporary shape on the npm side, and the
Rust side takes the same one:

```toml
poodle-specs = { version = "0.1.0", path = "../../../poodle/packages/contracts/components" }
```

**The long-term mechanism is git refs, not crates.io.** Poodle is untagged, so
path refs stand in until it carries a tag; at that point every consumer's path
ref swaps to a git ref together — npm and Cargo alike. Publishing to crates.io
was my recommendation and is not the plan.

### The CI consequence, stated

`ci.yml` says it "exists to prove a clean clone with no sibling checkouts, no
`[patch]` config, and no warm caches — the environment consumers of a tag are
in". A path ref contradicts that by construction, and so does the `file:` pin
that predates it: both jobs need a sibling Poodle to resolve.

So this crate does not make CI worse, it joins an existing gap — and the tag
that fixes it fixes both at once. Worth knowing rather than discovering during
a release.

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

## First domain — notifications, 2026-08-09

`longhorn-poodle` exists and projects `NotificationRecord` into Poodle's
`Toast`. Two tests, and it compiled against `poodle-specs` first try, which is
the useful signal: the two vocabularies mostly already agree.

### What building one domain exposed

**Longhorn has five severities; Poodle has four tones.** `Info`, `Success`,
`Warning` and `Error` map cleanly. `Critical` has nowhere to go — `Danger` is
as loud as Poodle gets — so it shares a tone with `Error`.

This is a real gap rather than an oversight on either side. Poodle's tones are
a *visual* vocabulary and four tints is a reasonable palette; Longhorn's
severities are an *operational* ladder and five levels is a reasonable ladder.
They simply do not line up, and no amount of care on either side would have
made them.

So `tone_for` returns a `ToneMapping` carrying `is_lossy`, true only for
`Critical`. A projection that returned the tone alone would make `Critical`
silently indistinguishable from `Error` at exactly the moment the distinction
matters. A surface that wants it back restores it in text or in an action,
because the tone cannot carry it.

**A toast holds one action; a record may carry several.** The first is
projected and the rest are reachable from the notification centre. That is a
presentation choice, not a loss of record, and it is stated here so nobody
reads it as a bug later.

Neither of these needed a Poodle change, so the stop condition did not fire.

### Parity against `longhorn-poodle-svelte`

| Domain | Rust | Note |
| --- | --- | --- |
| notifications | projected | toasts; severity collapse recorded above |
| config | not yet | |
| settings | not yet | |
| operations | not yet | |
| licence | not yet | |
| update | not yet | |
| layout | **deliberately absent** | not in the first target's needs; memo 021 |
| surfaces | **deliberately absent** | as above |
| transfer | **deliberately absent** | as above |

"Not yet" and "deliberately absent" are different claims and stay separated.

## Acceptance Criteria

- [x] the crate name is a recorded decision, not an inherited assumption
- [x] one domain is projected — notifications to `Toast`. Rendering through an adapter is the next step and needs an application to render into.
- [x] parity is stated per domain, with "deliberately absent" distinguished
  from "not yet"
- [x] no Poodle primitive is forked
- [x] Poodle contains no reference to Longhorn
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
