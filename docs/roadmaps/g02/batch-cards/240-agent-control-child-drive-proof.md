# 240 Agent Control Child Drive Proof

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.035
Governing refs: contract 022 (amended); Card 239; Cards 234/238
closeouts (packaged proof patterns); contracts 001, 003, 013
Depends on: Card 239
Auto-start next card: no — the lane closes here

## Objective

An agent drives an opted-in child webview end to end on a packaged app,
unfocused, and the teaching surfaces say exactly what shipped. Figmatic
gets its opt-in instructions.

## Scope

- **Proof app opt-in.** `examples/agent-control-proof` opts in one of
  its islands (the other stays closed — the refusal case must exist
  packaged too). Give the opted-in island interactive content worth
  driving: something clickable, a text field, and a drag target
  (marquee-like selection via drag is the Figmatic-shaped motion).
- **Packaged drive.** Extend the driver: against the packaged `.app`,
  unfocused throughout — snapshot the island, click in it, type in it,
  drag in it, `wait_for` a DOM predicate in it, `evaluate` in it, and
  screenshot the composed window showing the result. Then the refusal
  leg: the non-opted island answers typed `Unsupported`; a cross-webview
  ref answers `UnresolvedRef`. Focus samples and no-pointer evidence as
  in Cards 234/238; evidence committed.
- **Interleave.** Two clients, one driving the UI webview and one the
  opted-in island, no cross-talk, refs never crossing.
- **Truthfulness pass.** Skill: the `webview` addressing, the
  ref-scoping rule, the refusal meaning, and when to ask the operator
  for an opt-in rather than work around one (extends the existing stop
  rules). Guide: how an app opts in, and the security sentence — opting
  in asserts the child's content is the app's own to drive; do not opt
  in labels hosting third-party content (`longhorn-browser` views).
  Contract 022 Required Evidence: extended with this card's proof.
- **Handback and closeout.** Closeout names the landed revision and the
  exact Figmatic steps: bump, opt in `figmatic-preview` at mount,
  re-run the installer, automate the preview acceptance. Dated log;
  milestone and runway closeout to the orchestrator's pointer.

## Acceptance Criteria

- [ ] packaged drive green: all six tool motions inside the opted-in
      island, unfocused, evidence committed
- [ ] refusal leg green packaged: non-opted island typed `Unsupported`,
      cross-webview ref `UnresolvedRef`
- [ ] two-client UI/island interleave green
- [ ] skill, guide, and contract Required Evidence match the shipped
      mechanism; installer refresh picks up the skill; drift lock green
- [ ] release-absence scan green both directions on the final tree
- [ ] closeout names revision and Figmatic opt-in steps
- [ ] `effigy qa` passes

## Validation

`effigy qa`; the packaged drive on a macOS host (operator's display, no
focus or pointer theft); the release-absence scan.

## Stop Conditions

- driving the island requires focus, pointer, or trusted events for any
  motion — stop with evidence; that narrows the contract, operator
  decides;
- the island's untrusted drag cannot express the marquee-shaped motion
  the Figmatic need is about — record precisely what untrusted drag can
  and cannot do and stop before overclaiming.

## Continuation

g02.035 closes here. Figmatic opts in its preview and owns its own
automation; the orchestrator relays the revision and steps.
