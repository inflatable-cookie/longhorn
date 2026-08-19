# 237 Agent Control Skill Dogfood Proof

Status: done 2026-08-19
Owner: Longhorn maintainers
Roadmap: g02.033
Governing refs: contract 022; Cards 234-236; `examples/agent-control-proof`
Depends on: Card 236
Auto-start next card: no — the lane closes here

## Objective

The skill is proved the way it will be used: an agent, given only the
skill, launches the packaged proof app, discovers and connects to it,
and drives it — no OS focus, no pointer, no reading Longhorn source to
fill gaps.

## Scope

- **The run.** Execute the skill as written, as its audience: build and
  launch the packaged proof app (the guide's worked example), then
  follow the skill's own steps — finder script, connection, snapshot →
  click → type → wait_for → screenshot → command → listen — against the
  form-and-list UI. The skill text is the only allowed instruction
  source for the driving steps; the moment a step requires knowledge
  the skill does not contain, that is the finding.
- **Evidence.** A committed run record under
  `examples/agent-control-proof/evidence/<timestamp>-skill-dogfood/`:
  the exact commands run, the finder output (token redacted), the tool
  calls and their results, focus samples, and a note for every place
  the skill was ambiguous, wrong, or silent — even ones worked around.
- **The loop.** Gaps found feed straight back into the skill (and guide
  where composition-side) in this same card, then the drift check and a
  re-run of the affected steps confirm the fix. The card is done when a
  full pass completes with zero out-of-skill knowledge needed.
- **Milestone closeout.** g02.033 and lane surfaces close; the g02
  README block and generation-index pointer hand back to the
  orchestrator, who owns the per-app adoption handoff that follows.

## Acceptance Criteria

- [x] a full skill-only pass completes against the packaged app,
      unfocused throughout, evidence committed
- [x] every ambiguity found during the run is fixed in the skill/guide
      and the affected step re-proved — none recorded as "worked around"
- [x] finder script behaved per its contract on the real discovery dir
- [x] token appears nowhere in committed evidence
- [x] `effigy qa` passes (drift check included) on the final tree

## Validation

`effigy qa`; the committed dogfood run; `effigy doctor`.

## Stop Conditions

- the skill-only pass needs a capability the merged surface lacks —
  that is a contract/runway gap, not skill prose: stop and report;
- the run cannot be made honest without committing secret material —
  stop; evidence redaction is not optional.

## Closeout

Status: done 2026-08-19, same branch and worktree as Cards 235-236.

First pass: snapshot → click → type → wait_for → screenshot → command
succeeded unfocused; `subscriptions/listen` missed the first
`console.log` after subscribe. Cause: listen cursor used the ring's
`nextSeq` as `sinceSeq`, and the shim filters `seq > sinceSeq`, so the
next assigned event was dropped. Card 234's two-client e2e hid this
(two evaluates, the second event passed the cursor). Reported; operator
authorised the fix. Cursor is now the last delivered event seq;
conformance `listen_delivers_the_first_event_after_subscribe` locks it.

Skill gained the ack-vs-`resources/updated` distinction and the
`list_commands` / `proof:ping` note. Re-run green:
`examples/agent-control-proof/evidence/2026-08-19T22-06-06-skill-dogfood/`
(`appHeldFocus: false`, token redacted, findings empty).

## Continuation

g02.033 closes here. Next: per-app adoption cards in the five consumer
repos (compose per the guide, install the skill via effigy); the stdio
proxy client stays in the candidate runway.
