# 174 GPUI Worked Example

Status: complete
Completed: 2026-08-10
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

## Ready

Both blockers are closed. Card 172 settled where it lives — a sibling of the
existing prototypes, covered by `effigy check:prototypes`. Card 173's guide
says what it must show, and the two should be readable against each other.

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

## Result

`prototypes/gpui-composition`, a seventh prototype picked up by
`effigy check:prototypes` with no selector change. It draws: three toasts from
a real ledger, the withheld capabilities with their reasons, and a date the
injected `HostServices` produced.

### It confirmed something a test could not

`Critical: Severity says itself` is visible on the surface. The D5 prefix was
asserted by test when it landed, and the display was asleep when I tried to
look; this is the first time it has been seen. It reads correctly beside the
two toasts that carry no prefix.

### It found a bug in itself immediately

The first run rendered "assembled 10 August 2025" for a date meant to be 2026.
The civil-date arithmetic was right and the constant was wrong — which is the
cheapest possible version of the lesson that a date is a decision an
application makes and gets wrong.

### Two deliberate omissions, both stated in the example

**The window backend is not restated.** Writing a `GpuiWindowBackend` over
`gpui::PlatformWindow` is the neighbouring prototype's subject; this one opens
its window through gpui directly and says so. Duplicating it would have made
the example about the binding rather than about the composition.

**`format_timestamp` is written out rather than pulled from a date crate.**
Which crate is the application's decision, and the guide's whole first section
is that Longhorn does not make it. Hinnant's `civil_from_days` is short enough
to read and makes the point better than a dependency would.

## Do Not

- Reuse `PlainHostServices`. It exists for tests and is named to discourage
  exactly this.
- Grow it into a demo of six domains. The guide is the surface; this is the
  proof the guide compiles.

## Acceptance Criteria

- [x] the example is covered by `check:prototypes` with no selector change
- [x] it follows the guide's order, its section comments naming the steps
- [x] it supplies the seams the guide names that a composition needs;
  the window-backend seams are the neighbouring prototype's and it says so

## Evidence Required

- [x] the example, green under `effigy check:prototypes`
- [x] a run, inspected: three toasts, the `Critical:` prefix, the withheld
  capabilities, and a date from the injected services

## Stop Conditions

- the guide's assembly turns out to need surface Longhorn does not have, which
  is Card 173's stop condition arriving late
