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

**Card 172.** Where this lives depends entirely on where a `gpui` build runs.
If the answer is a nightly selector, this is a workspace member excluded from
`qa`. If it is a release gate, it is an example built at tag time. If it is
hand-run, this card should probably not exist at all and the guide's example
stays illustrative — which is a legitimate outcome and the reason 172 comes
first.

## Scope

- one application: a window, a projected domain drawn in it, `HostServices`
  supplied for real
- the minimum that exercises the assembly, not a demonstration of every domain

## Steps

1. Site it per Card 172's decision.
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

- the example compiles under the cadence Card 172 chose
- it follows the guide's order, so the two can be read together
- it supplies every seam the guide names

## Evidence Required

- the example, building under its chosen cadence
- one deliberate signature break proving the cadence catches it

## Stop Conditions

- Card 172 chose "hand-run", in which case reconsider whether this card should
  exist rather than adding a second ungated artefact
