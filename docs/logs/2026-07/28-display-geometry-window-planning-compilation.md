# Display Geometry And Window Planning Compilation

Date: 2026-07-28
State: complete planning batch

## Outcome

- revalidated display and window evidence in Loophole, Nucleus, and Soundcheck
- sharpened contract 009 scale, rounding, and identity-allocation rules
- compiled `g01.003` into four dependency-ordered cards
- made Card 013 the sole ready implementation lane
- preserved Cards 014 through 016 as the visible runway
- kept `g01.004` host mutation and packaged evidence outside the pure lane

## Donor Evidence

Loophole retains the strongest inventory and correlation system:

- canonical and process-local display ids remain separate
- geometry/scale, geometry-only, and rearranged size evidence carry confidence
- labels and remembered client ids survive registry updates
- configured display fallbacks and per-display geometry are pure window inputs

Nucleus retains two complementary proofs:

- `nucleus-workspaces` has no-Surface display records and pure fallback planning
- desktop restore selects saved display, largest intersection, primary, then
  first available display before clamping to the work area

Soundcheck retains the minimal case:

- one remembered window
- outer origin plus inner content size
- primary/first fallback
- explicit minimum size and work-area clamp
- host-owned debounce and close flush

No donor repository was modified.

## Compiled Runway

1. Card 013 — typed coordinate and geometry foundation in `longhorn-core`
2. Card 014 — known/observed display inventory and correlation in
   `longhorn-display`
3. Card 015 — pure placement and fallback resolution in
   `longhorn-windowing`
4. Card 016 — desired/live diff operations and apply generations in
   `longhorn-windowing`

Card 013 is ready. Later cards remain planned until their dependency closes.
No card auto-starts its successor.

## Contract Decisions

- durable scale evidence uses positive integer thousandths
- physical/logical conversion always names rounding
- overflow and invalid client coordinates fail typed
- new `DisplayId` values come from an injected allocator
- host ids, hardware keys, fingerprints, and enumeration order remain evidence
- ambiguous weak matches never bind automatically
- minimum size, minimum visibility, home adoption, focus, and protected-primary
  choices remain explicit policy inputs

## Boundary

`g01.003` owns pure ids, geometry, correlation, placement, and diff planning.
It does not own Tauri observation or mutation, platform key acquisition, event
settling, debounce, persistence, ambiguity UI, layout containers, Surfaces,
drag, Svelte, or Poodle.

## Validation

- roadmap, contract, architecture, research, and front-door links passed
- Effigy Northstar checks passed
- Effigy Doctor reported warning-only size findings and zero errors

## Posture

`strict-ready`

## Next

Execute Card 013. Stop before Card 014.
