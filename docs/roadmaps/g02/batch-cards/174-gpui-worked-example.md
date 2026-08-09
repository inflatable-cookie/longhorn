# 174 GPUI Worked Example

Status: blocked
Owner: Tom
Roadmap: g02.015
Governing refs: contract 020; contract 013
Depends on: Card 172; Card 173
Auto-start next card: no

## Objective

The composition guide's assembly as something that compiles.

## Why this exists

Prose assembly rots. The Tauri side already learned this — `examples/` holds
five proof applications that are workspace members precisely so a signature
change breaks them loudly. GPUI has one prototype, outside every gate, and it
was broken by a signature change in the same session that introduced it.

## Blocked on

**Card 173**, for what the example must show. Card 172 is done and settled
where it lives: a sibling of the existing prototypes, covered by
`effigy check:prototypes`, checked outside `qa` and built before a tag.

## Scope

- one application: a window, a projected domain drawn in it, `HostServices`
  supplied for real
- the minimum that exercises the assembly, not a demonstration of every domain

## Steps

1. Site it under `prototypes/`, alongside the existing six, so
   `check:prototypes` picks it up with no selector change.
2. Assemble exactly what Card 173's guide describes, in the same order, so a
   reader can follow one against the other.
3. Draw one domain end to end. Notifications is the cheapest — it needs a
   ledger and nothing else — and `prototypes/gpui-windowing/src/bin/render.rs`
   already shows the shape.
4. Supply `HostServices` honestly. Real date formatting, not
   `PlainHostServices`; the point is to show what an application must decide.

## Do Not

- Reuse `PlainHostServices`. It exists for tests and is named to discourage
  exactly this.
- Grow it into a demo of six domains. The guide is the surface; this is the
  proof the guide compiles.

## Acceptance Criteria

- the example is covered by `check:prototypes` with no selector change
- it follows the guide's order, so the two can be read together
- it supplies every seam the guide names

## Evidence Required

- the example, green under `effigy check:prototypes`
- Card 172 already proved the selector catches a signature break; this needs only to be inside it

## Stop Conditions

- the guide's assembly turns out to need surface Longhorn does not have, which
  is Card 173's stop condition arriving late
