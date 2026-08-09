# 163 GPUI Host Adapter

Status: complete
Owner: Tom
Roadmap: g02.012 batch 3
Governing refs: contract 020; research memo 021
Depends on: Card 161
Auto-start next card: no
Completed: 2026-08-09

## Objective

Implement the minimal GPUI host adapter, and use it to validate or refute
contract 020. An adapter architecture with one implementation is a
hypothesis; this is the experiment.

## Scope

- `longhorn-gpui-windowing`: window create/destroy/observe, placement
  application, lifecycle events, close handling, quiescence participation
- display facts with scale factors
- whatever contract 020 turns out to have got wrong

## Steps

1. Write the adapter against contract 020 as written, without amending it
   first. The point is to find where the contract is Tauri-shaped, and
   editing it in advance destroys that signal.
2. Execute the same pure placement plans `longhorn-windowing` produces. The
   planning is shared; only execution differs.
3. Participate in the restart interlock. The host reports its own
   outstanding work.
4. Record every place the contract had to bend, with which backend's
   assumption caused it.
5. Amend contract 020 from those findings, then re-check Tauri still
   satisfies it.

## Progress — 2026-08-09

`longhorn-gpui-windowing` exists: 29 tests, `effigy qa` green. It depends on
no `gpui`. `prototypes/gpui-windowing` binds its seam to `gpui` 0.2.2 and
compiles, which is what makes the seam's shape evidence rather than a guess.

### Where the crate lives, and why not in the gate

`gpui` pulls several hundred transitive crates and a Metal shader build.
Putting it in the workspace taxes `lint:rust`, `lint:rust:features`,
`test:rust` and `docs:rust` on every run, for one adapter. So the binding is
an excluded `prototypes/` crate with its own lock, exactly as
`prototypes/native-content` is, and the workspace crate's behavioural tests
run against an in-memory host implementing precisely `gpui::PlatformWindow`'s
surface.

Poodle drew the same line first: `packages/gpui/adapter` has no `gpui`
dependency; only `packages/gpui/preview` does.

The cost is stated: nothing in `effigy qa` would catch `gpui` changing under
the seam. A GPUI proof application is what closes that, and it is not this
card.

### The bends, with the backend that caused each

| # | Bend | Cause |
| --- | --- | --- |
| 1 | `HostCapability::MoveResize` is a compound; GPUI has resize and no move, so it must withhold both | GPUI |
| 2 | Placement is creation-time; the plan's create-neutral-then-mutate order cannot be executed literally | GPUI |
| 3 | No runtime show, hide, or visibility query; a window desired hidden is unreachable | GPUI |
| 4 | Maximize is `zoom_window`, a toggle; absolute state needs read-then-toggle and is not atomic | GPUI |
| 5 | `PlatformDisplay` reports id, uuid and logical bounds — no scale, no work area, no built-in flag — and `DisplayFacts` requires two of those | GPUI |
| 6 | No standalone scale-change event; one `on_resize` becomes two Longhorn events | GPUI |
| 7 | `on_should_close` demands a synchronous answer; there is no handle to hold | GPUI |
| 8 | `WindowDiffInput::desired_windows` was `pub(crate)`, so a host that must know final placement before a window exists could not read it | Tauri — it never needed to ask |
| 9 | The host seam is `Send + Sync` throughout, with `Arc` + `Mutex` and flushes on a blocking pool | Tauri — GPUI is main-thread-only and `!Send` |
| 10 | The capture seam threads `retained_normal` through every call | Tauri — it cannot report a maximized window's normal geometry; GPUI can |
| 11 | Display correlation is built on name plus geometry, with an ambiguity error | Tauri — GPUI has a UUID stable across restarts |

Two more, recorded but not contract changes:

- `CountingProbe`, `transfer_session_probe` and `operation_probe` sit in
  `longhorn-tauri-update` and reference no Tauri. Same leak class Card 161
  closed for windowing, still open here.
- `ScreenDip` is an integer. Tauri reports `f64` and GPUI reports `f32`, so
  **both** hosts round. That is Longhorn's own choice, not a host's, and it
  should not be mistaken for a bend.

### What was fixed here, and what was not

Not fixed: bend 8. Making `WindowDiffInput::desired_windows` public is the
right change — additive, no Tauri impact — and it was made and then reverted.
`crates/longhorn-windowing` is inside the source set the greenfield receipt
freezes at `b7c719c0`, so a one-line visibility change turns
`proof:artifacts` red, and regenerating that receipt belongs to the release
runway another thread is mid-flight on. `execute_gpui_window_apply` therefore
takes `desired_windows` as a parameter alongside the input that already
contains them, with the reason at the call site. It is the freeze that
deferred this, not a technical obstacle.

Answered: bend 9 settles Card 161's open question. The seven pure port types
should **not** move wholesale into `longhorn-windowing` — their `Send + Sync`
bounds are host policy, and a GPUI host would have to satisfy a constraint it
has no reason to. Contract 020 now says so under "What A Host Owns".

Not fixed: bend 1. Splitting `MoveResize` into `Move` and `Resize` changes
the pure planner's vocabulary, `WindowOperation`, and both adapters. It is
recorded in contract 020's divergence register with its consequence and
scheduled as its own card. Until then a GPUI window is placed at creation and
never again, and the adapter names that per window rather than hiding it.

### Stop condition — not met

No amendment breaks the Tauri adapter. Every one is a loosening, a
restatement, or an additive visibility change; `longhorn-tauri-windowing` is
untouched and its suite passes unchanged. The one change that would break it
is the `MoveResize` split, which is why it is scheduled rather than taken.

### Contract 020 is not complete on this evidence

Stated in the contract itself. Display facts with scale factors are
**unsatisfiable from a GPUI host alone**; the adapter records which facts were
missing instead of inventing them. Platform directories are not exercised on
GPUI at all. And no backend has proved multi-window placement, cross-window
transfer, or lifecycle teardown under load — the first GPUI target is a small
audio-conversion application that exercises none of the three, and those are
exactly where a single-host contract is most likely to have leaked.

## Acceptance Criteria

- a GPUI window is created, placed from a shared plan, and observed
- lifecycle events translate into Longhorn's vocabulary
- close handling defers to the restart interlock
- every contract-020 requirement is either satisfied or recorded as
  unsatisfiable with its reason
- Tauri still satisfies contract 020 after any amendment

## Evidence Required

- the adapter, and the list of contract bends with their causes
- a re-check of the Tauri adapter against the amended contract

## Stop Conditions

- contract 020 needs a change that would break the Tauri adapter, in which
  case the divergence is stated per backend rather than resolved by
  forcing one to match the other

## Notes

The first real GPUI target is a small audio-conversion application — a
product replacing a subscription, so it exercises licence and update, which
are exactly the two delegated-capability gaps.

It will **not** exercise multi-window placement, cross-window transfer, or
lifecycle teardown under load. Those are where Tauri's assumptions are most
likely to have leaked, so contract 020 must not be declared complete on this
card's evidence.

## Next Task

Close g02.012 when the contract has been amended and both backends
re-checked.
