# g02.015 GPUI Composition And Live Evidence

Status: ready
Owner: Tom
Updated: 2026-08-09
Governing refs: contract 020; contract 013; contract 012; memo 021; memo 022
Depends on: g02.012 (complete)

## Outcome

A GPUI application can be assembled from Longhorn's published surfaces by
following one guide, and the two claims contract 020 still holds at
in-memory-only are proved against a running window.

## Generation Runway

Fifteenth g02 milestone, and the first that exists to *use* the tier rather
than build it. g02.012 established two first-class hosts; cards 169-171 built
the projection tier and proved it against both backends. Everything since has
been evidence, and the evidence has hit the ceiling a real application removes.

## Planning Gaps

Named here rather than buried, because one of them governs the whole shape.

- ~~**Heavyweight host SDKs have no in-gate home.**~~ Closed by Card 172:
  `check:prototypes`, outside `qa`, in the release gates. The answer was an
  Effigy change rather than a workspace change, as anticipated.
- **Longhorn does not own the first product target.** Memo 021 names a small
  audio-conversion application, and the authority map puts Soundcheck's
  workflows and desktop policy in Soundcheck. This milestone owns the
  *composition surface and the in-repo evidence*; the consumer conversion is
  the consumer's, and this roadmap must not imply otherwise.

## Execution Plan

### Batch 1. Decide where a GPUI build runs

- [x] [Card 172](batch-cards/172-gpui-build-cadence.md) chose a named selector
  wired to release. `effigy check:prototypes` covers all six prototypes in 1.3s
  warm, outside `qa`. Measured first: `gpui` is 757 packages and 3.3 GiB linked,
  but 37s cold — heavy in disk and CPU, not wall clock, so what kept the
  exclusion is that the cost lands on four selectors and every cache rather
  than that it is large.

### Batch 2. Composition surface

- [x] [Card 173](batch-cards/173-gpui-composition-guide.md) —
  `docs/guides/gpui-composition.md`. Leads with the three platform facilities a
  webview gives away and GPUI does not, then names all eight seams in one list.
  Nothing invented; the stop condition did not fire.
- [x] [Card 174](batch-cards/174-gpui-worked-example.md) —
  `prototypes/gpui-composition`, gated by `check:prototypes` with no selector
  change. It confirmed the `Critical:` prefix on a real surface, which no test
  could, and found its own date bug on the first run.

Batch 2 closes the milestone's composition half: a guide, an example that
compiles, and the two readable against each other.

### Batch 3. Live evidence for the last two ceilings

- [x] [Card 175](batch-cards/175-live-cross-window-drag.md) — three real drags
  through the window server: to the other window in both directions, and to an
  empty display off both. Found that `on_mouse_up` never fires for a
  cross-window release, and that a window cannot be observed from inside its
  own event callback.
- [ ] [Card 176](batch-cards/176-live-teardown-under-load.md): a real teardown
  with a real flush in flight. **Real store landed** — 18-20ms per atomic
  write — and it found a window that grew by its titlebar every restart. The
  **Answered.** A window moved just before it closes loses its placement: the
  capture stages, the close is permitted, no flush reaches the store. It is the
  shared coordinator, so Tauri carries it too. The fix — a close that forces
  its own flush and waits — changes both hosts and is an operator decision.

## Dependency Shape

```text
contract 020 (ceilings stated)      memo 022 (divergences closed)
 └─ 015 GPUI composition and live evidence
     ├─ 172 build cadence            (blocks 174, 175, 176)
     ├─ 173 composition guide        (independent)
     ├─ 174 worked example           (needs 172, 173)
     ├─ 175 live cross-window drag   (needs 174)
     └─ 176 live teardown under load (needs 174)
```

172 first and alone: siting an example before deciding where it builds is how
`prototypes/` became ungated in the first place.

## Goals

- [x] a GPUI application can be assembled without reading adapter source
- [x] every seam a GPUI application must supply is named in one place
- [ ] contract 020 holds no claim proved only in memory
- [ ] the GPUI binding is built by something other than a person remembering to

## Acceptance Criteria

- [ ] the composition guide assembles a window, a projection, and
  `HostServices`, and its example compiles under whatever cadence Batch 1 chose
- [x] a drag released over a second real window resolves to that window
- [x] a window torn down with a flush genuinely in flight either completes it
  or refuses the close, and the proof says which — it does neither: it stages,
  permits the close, and never writes
- [ ] no claim in contract 020's current-state table reads "in-memory only"
  without a stated reason it cannot be more

## Explicit Non-goals

- Converting Soundcheck, Loophole, or Nucleus. The authority map puts consumer
  workflows in consumer repos, and a Longhorn roadmap does not schedule them.
- A GPUI application framework. Longhorn supplies adapters and projections; the
  application is the application's.
- Moving `gpui` into the default workspace. Batch 1 may decide a slower cadence
  is the answer, and that is not the same thing.

## Next Task

Card 176's three observations. All need a real window closed while the example
is frontmost, and keeping it there proved to be the hard part — a titlebar drag
sent it behind another application twice.

The one Card 175 criterion left open is the same shape: a window moved
mid-drag, which needs the window to stay in front while it is moved.

## Planning Checkpoint

After Batch 2. If the composition guide cannot be written without inventing
surface Longhorn does not have, that is a contract 020 gap and this milestone
pauses rather than filling it in with plausible API.
