# g02.035 Agent Control Child-webview Targeting

Status: ready
Owner: Longhorn maintainers
Created: 2026-08-20
Governing refs: contract 022 (amended 2026-08-20: opt-in child-webview
semantic targets, default closed); the Figmatic triage note
(`figmatic:docs/triage/20260820-162239-agent-control-preview-input-gap.md`);
contracts 001, 006, 012, 013
Depends on: g02.034 complete (`e135d725`)

## Outcome

An application can name child-webview labels as semantic targets at
mount, and an agent can then snapshot, click, type, drag, wait, and
evaluate inside those children — with webview-scoped refs, explicit
webview addressing, and everything not opted in answering typed
`Unsupported`. Figmatic opts in its preview and agents stop needing the
operator's hands for marquee selection.

## Generation Runway

- [ ] [Card 239](batch-cards/239-agent-control-webview-targeting.md) —
      vocabulary and plumbing: opt-in config, `webview` addressing,
      webview-scoped refs, typed refusals, conformance fixtures.
- [ ] [Card 240](batch-cards/240-agent-control-child-drive-proof.md) —
      packaged proof driving an opted-in island unfocused; skill, guide,
      and contract truthfulness; Figmatic handback.

## Acceptance

- Default closed: with no opt-in, behavior is byte-identical to today
  and the old wire shapes still work unchanged.
- An opted-in child is drivable end to end (snapshot → click → drag →
  wait_for → evaluate) unfocused, proved packaged.
- A ref never resolves against a webview it did not come from; a
  non-opted-in or unknown webview target fails typed naming why.
- The skill teaches the addressing and the refusal; the drift lock
  still passes; release absence unchanged.
- Figmatic receives the landed revision and opt-in instructions.

## Next Task

After completion: Figmatic opts in `figmatic-preview` and automates its
preview acceptance; remaining consumer adoptions continue.
