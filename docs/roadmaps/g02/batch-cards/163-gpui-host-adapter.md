# 163 GPUI Host Adapter

Status: ready
Owner: Tom
Roadmap: g02.012 batch 3
Governing refs: contract 020; research memo 021
Depends on: Card 161
Auto-start next card: no

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
