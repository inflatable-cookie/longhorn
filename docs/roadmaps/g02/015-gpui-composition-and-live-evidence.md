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

- **Heavyweight host SDKs have no in-gate home.** `gpui` adds several hundred
  transitive crates and a Metal shader build to every Rust selector, so the
  binding lives in `prototypes/`, outside every gate, verified by hand.
  Recorded in `PAPERCUTS.md`. Batch 1 decides this before anything is built on
  top of it; if the answer is "a slower cadence", that is an Effigy change, not
  a workspace change.
- **Longhorn does not own the first product target.** Memo 021 names a small
  audio-conversion application, and the authority map puts Soundcheck's
  workflows and desktop policy in Soundcheck. This milestone owns the
  *composition surface and the in-repo evidence*; the consumer conversion is
  the consumer's, and this roadmap must not imply otherwise.

## Execution Plan

### Batch 1. Decide where a GPUI build runs

- [ ] Card 172: choose the cadence for prototype and example builds — nightly
  Effigy selector, release gate, or explicitly hand-run — and make whichever it
  is real. Until this lands, everything below is verified by hand.

### Batch 2. Composition surface

- [ ] Card 173: a composition guide with one worked assembly — host adapter,
  projection tier, `HostServices`, and the seams a GPUI application must
  supply that a Tauri one never had to think about.
- [ ] Card 174: the guide's example as a compiling artifact rather than prose,
  sited by Batch 1's decision.

### Batch 3. Live evidence for the last two ceilings

- [ ] Card 175: a real drag across two real GPUI windows — mouse events bound
  to a session, released over another window — closing contract 020's last
  stated ceiling.
- [ ] Card 176: a real teardown with a real flush in flight, which the
  thirteen-window in-memory proof deliberately does not cover.

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

- [ ] a GPUI application can be assembled without reading adapter source
- [ ] every seam a GPUI application must supply is named in one place
- [ ] contract 020 holds no claim proved only in memory
- [ ] the GPUI binding is built by something other than a person remembering to

## Acceptance Criteria

- [ ] the composition guide assembles a window, a projection, and
  `HostServices`, and its example compiles under whatever cadence Batch 1 chose
- [ ] a drag released over a second real window resolves to that window
- [ ] a window torn down with a flush genuinely in flight either completes it
  or refuses the close, and the proof says which
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

Card 172. It is the only one of the five that needs no new evidence and blocks
three of the other four.

## Planning Checkpoint

After Batch 2. If the composition guide cannot be written without inventing
surface Longhorn does not have, that is a contract 020 gap and this milestone
pauses rather than filling it in with plausible API.
