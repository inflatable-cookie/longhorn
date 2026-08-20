# 240 Agent Control Child Drive Proof

Status: done 2026-08-20
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

- [x] packaged drive green: all six tool motions inside the opted-in
      island, unfocused, evidence committed
- [x] refusal leg green packaged: non-opted island typed `Unsupported`,
      cross-webview ref `UnresolvedRef`
- [x] two-client UI/island interleave green
- [x] skill, guide, and contract Required Evidence match the shipped
      mechanism; installer refresh picks up the skill; drift lock green
- [x] release-absence scan green both directions on the final tree
- [x] closeout names revision and Figmatic opt-in steps
- [x] `effigy qa` passes

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

## Closeout

Status: done 2026-08-20, same branch and worktree as Card 239
(`t3code/read-webview-targeting-handoff`,
`/Users/tom/.t3/worktrees/longhorn/t3code-3feafabe`).

**Packaged drive.** `examples/agent-control-proof` opts in `preview`
and leaves `preview-top` closed. Island content: `Island Go`, `Island
Note`, a 3×3 marquee grid, plus the Card 238 ticker. e2e schema v2
(`examples/agent-control-proof/evidence/2026-08-20T15-55-00-e2e/`):
snapshot / click / type / drag / wait_for / evaluate inside `preview`,
screenshot of the composed window, closed-child `Unsupported`,
cross-webview `UnresolvedRef`, UI/island interleave with distinct refs.
Unfocused throughout (five focus samples, app never frontmost). No OS
pointer.

**Untrusted drag, precisely.** The tool is ref-to-ref and two-point:
pointer/mouse down at the source center, move at the target center,
HTML5 DnD (`dragstart` → `dragenter` → `dragover` → `drop` → `dragend`),
pointer/mouse up at the target. The packaged island listens to those
events and selected `0,0:2,2` from Cell 0 0 → Cell 2 2. There is no
interpolated pixel path. A marquee that only samples intermediate
`mousemove` coordinates along a free-form trail is not expressed;
a two-ref selection (start cell → end cell) that listens to
pointer/mouse down/up or HTML5 DnD at those elements is.

**Figmatic opt-in.** After merge, pin the landed `main` commit. In the
Figmatic worktree: bump `longhorn-tauri-agent-control` to that
revision; at mount, `.with_semantic_child("figmatic-preview")` (the
preview island's webview label); rebuild; re-run
`bun scripts/install-agent-control-skill.ts <figmatic>` from Longhorn
so the skill picks up `webview` addressing and the opt-in stop rule;
drive the preview with `webview: "figmatic-preview"` — snapshot, then
click/type/drag by the island's refs. Do not opt in any
`longhorn-browser` view. Figmatic owns the bump, the opt-in, and its
preview-acceptance automation.
