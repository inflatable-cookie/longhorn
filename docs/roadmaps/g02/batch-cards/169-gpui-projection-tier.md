# 169 GPUI Projection Tier

Status: ready
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

## The name

Memo 021 flagged this as an open question and it was answered sideways: Card
164 states "the Rust sibling will be `longhorn-poodle-gpui`" as a passing
justification for naming the TypeScript package `longhorn-poodle-svelte`.
That is a decision by assertion. **This card either confirms it deliberately
or changes it**, and records which.

The argument for confirming: the pairing is already visible in
`longhorn-poodle-svelte`, the TypeScript name shipped on that basis, and
renaming after publication is a breaking change across every consumer.

## Scope

- `longhorn-poodle-gpui`: projections from Longhorn domains to Poodle GPUI
  specs
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

1. Decide the name, deliberately, and record the reasoning either way.
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
   `poodle-gpui`, and Poodle keeps no reference to Longhorn.

## Where the gpui dependency goes

Card 163 kept `gpui` out of the workspace, because it pulls several hundred
transitive crates and a Metal shader build, and put the real binding in an
excluded `prototypes/` crate. A projection tier renders, so it cannot dodge
`gpui` the same way — `poodle-gpui` itself takes no `gpui` dependency, but
anything that produces elements eventually does.

**This is the card's main structural question and it should be answered
before code.** Options: an excluded crate like the prototype, an optional
feature, or accepting the build cost in the workspace. The open papercut
"heavyweight host SDKs have no in-gate home" is the same problem, and this
card is the one that forces it.

## Acceptance Criteria

- the crate name is a recorded decision, not an inherited assumption
- one domain is projected end to end and renders in the GPUI preview
- parity is stated per domain, with "deliberately absent" distinguished from
  "not yet"
- no Poodle primitive is forked
- Poodle contains no reference to Longhorn
- the `gpui` dependency's home is decided and its cost to `effigy qa` is
  stated

## Evidence Required

- the projection for one domain, and a running window that draws it
- the parity table
- a statement of what the `gpui` dependency costs whichever lane now carries
  it

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
