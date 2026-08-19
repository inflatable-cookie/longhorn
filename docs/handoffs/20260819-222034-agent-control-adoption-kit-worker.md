---
title: Agent control adoption kit worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-19
updated: 2026-08-19
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260819-222034-agent-control-adoption-kit-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

The agent-control build runway finished today (PRs 2-5): contract 022 is
evidence-closed, and a dev-featured Longhorn app serves a stateless MCP
control surface an agent can drive without OS focus. Your run is
milestone g02.033, Cards 235-237 — the adoption kit that makes the five
consumer apps able to pick this up cleanly: a composition guide for the
Rust half, a canonical skill (plus finder script) for the agent half, a
one-command effigy install with a qa drift lock, and a dogfood pass
where the skill is proved by following it.

This file is your complete brief; you don't need the planning
transcript.

## Why It Matters

The feature only pays off when an agent in a consumer repo reaches for
it instead of OS computer use. That takes two artifacts nobody has yet:
a guide a consumer card can execute blind, and a skill that teaches an
agent the surface's real contracts — refs go stale honestly, waits are
DOM-relative, input is untrusted, events are resources. The drift lock
is what keeps five installed copies truthful as the surface evolves;
the dogfood pass is what keeps the skill honest on day one.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `982eff16945137728eab5b5430835a8067b30c5c`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at
  that commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff file.
- **Worker mode:** implementation worker dispatched by the orchestrator;
  this handoff activates the worker-only worktree preflight.
- **Planning artifacts included at the base:** contract 022
  (evidence-closed), the four merged agent-control lanes,
  `examples/agent-control-proof` with its e2e driver and evidence,
  milestone g02.033 (ready), Cards 235-237.
- **Worker branch:** `worker/235-agent-control-adoption-kit` — the
  branch number is a label; the cards are 235, 236, and 237.
- **Worker worktree:** launcher-provided when available; otherwise a
  unique worktree under `/Users/tom/Dev/worktrees` (operator-confirmed
  container).
- **Worktree creation command (manual fallback only):**
  `git worktree add /Users/tom/Dev/worktrees/longhorn-235 -b worker/235-agent-control-adoption-kit origin/main`
- **Worker worktree policy:** first use the clean, dedicated, non-`main`
  registered worktree supplied by the launcher, even if its generated
  path or branch differs from these placeholders. Record the actual
  path/branch and never create a second worktree for that reason. Only
  if the current context is `main`, dirty, unregistered, or otherwise
  unusable, use the named command above. `.agents.local.env` still does
  not exist (PAPERCUTS records why); the container above is the
  operator's standing answer. Never use `/tmp`, `TMPDIR`, or a guessed
  path.
- **Active spec lane:** none open. Contract 022 is promoted background;
  this lane documents it and must not amend it.
- **Roadmap milestone:** g02.033
  (`docs/roadmaps/g02/033-agent-control-adoption-kit.md`)
- **Ready cards, in order:**
  `docs/roadmaps/g02/batch-cards/235-agent-control-guide-and-skill.md`,
  `docs/roadmaps/g02/batch-cards/236-agent-control-skill-install-and-drift.md`,
  `docs/roadmaps/g02/batch-cards/237-agent-control-skill-dogfood-proof.md`
- **Allowed runway:** Cards 235-237, one PR.
- **Remaining card budget:** three cards; then stop and report.
- **Dispatch topology:** serial — 236 locks 235's skill; 237 proves it.
- **Parallel safety check:** not applicable; one lane.
- **Canonical refs:** `AGENTS.md`; contract 022
  (`docs/contracts/022-agent-app-control.md`); contracts 001, 003
  (consumer boundary — this lane's most load-bearing ref), 012;
  `docs/guides/README.md` for guide shape; the Card 230-234 closeouts
  for the mechanics the guide and skill describe; the closing log
  `docs/logs/2026-08/19-agent-control-runway-closeout.md`.
- **Expected seam-touches, pre-authorized:** `effigy.toml` (the two new
  selectors), `scripts/` or `skills/agent-control/scripts/` per the
  cards, `docs/guides/` and its README, the new top-level
  `skills/agent-control/`, and — for Card 236's single source of truth —
  a read-only export of the tool-name list from
  `longhorn-agent-control` if the conformance suite's list is not
  already reachable. Anything beyond that list (contract text, tool
  behavior, plugin code, consumer repos) stays report-first: tell the
  operator before the edit, not in the PR body. The build runway logged
  three lanes of correct-but-unreported fixes; this handoff scopes the
  expected touches in precisely so that rule can be strict about the
  rest.
- **Model capability profile:** capable coding model; the skill's
  writing quality matters as much as the code — it will be read by
  agents under pressure to fall back to old habits.
- **Tool/runtime restrictions:** macOS host; Card 237 builds and runs
  the packaged proof app on the operator's display.
- **Required validation:** `effigy qa` (with the new drift check in the
  board); finder and install fixtures; the committed dogfood run;
  `effigy doctor` for orientation.
- **PR base/head:** `main` ← `worker/235-agent-control-adoption-kit`
  (or the launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** the guide, the skill and finder script, the install
  selector and drift check with their fixtures, the dogfood run and the
  skill/guide fixes it forces, and the card/milestone/log surfaces.
- **Out of scope:** consumer repos (the install selector is proved
  against a scratch/temp repo, never a sibling app — contract 003);
  contract 022 edits; any change to tool behavior (if the skill can't
  describe what exists, that's a gap report); the stdio proxy client
  (candidate runway); new capture or platform work.
- Skills and scripts are executable surfaces: no placeholder values
  that break when copied (AGENTS.md redaction rule), and the instance
  token never lands in committed evidence or diagnostics output.
- Do not invent architecture, widen the roadmap, or settle an
  unresolved product/API/persistence/security decision.
- Shared checkout discipline: stage by explicit path, never
  `git add -A`, never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout.
- Do not merge the PR. Merge remains a separate operator-authorised
  action.

## Important Context

- **Planning lineage:** memo 024 → contract 022 → PRs 2-5 → this lane.
  The five logs under `docs/logs/2026-08/19-agent-control-*.md` are the
  full history; the runway-closeout log carries the standing findings
  this handoff already acts on.
- **Why these cards are ready:** everything the guide and skill
  describe is merged and proved; nothing here waits on a decision. The
  one seam that might need code — exporting the tool-name list for the
  drift check — is pre-authorized above.
- **Decisions already made (don't relitigate):** skill lives at
  `skills/agent-control/` in Longhorn and mirrors into consumer
  `.claude/skills/` via effigy, operator-invoked, never automatic;
  connection is dynamic (finder → `claude mcp add` with bearer header)
  with raw stateless POST as the fallback; the drift lock shares the
  conformance suite's vocabulary source rather than retyping it; the
  dogfood pass allows zero out-of-skill knowledge — a workaround is a
  finding to fix, not a note to record.
- **Facts the skill must carry correctly** (from the merged evidence,
  not from memory): discovery lives under the contract 004 state root
  (`.../longhorn/state/agent-control/<app-id>-<pid>.json` — check the
  real path shape in `longhorn-agent-control::discovery`); stale = dead
  pid; events are `resources/updated` on
  `longhorn://agent-control/{console,error,navigation}` with a drop
  counter on the resource body; rAF stalls unfocused so waits are
  DOM-relative by design; capture is fresh occluded and minimized,
  macOS only; native menus/dialogs go through `command`.
- **Dogfood logistics:** Card 237 runs the packaged proof app on the
  operator's display — coordinate the moment through the operator, same
  as the last two lanes. The e2e driver from Card 234 shows the
  launch/discovery mechanics, but the dogfood pass must follow the
  skill's steps, not the driver's.
- **Report after:** (1) Card 235's guide + skill + finder fixtures
  green; (2) Card 236's install and drift fixtures green with `qa`;
  (3) Card 237's dogfood evidence committed; then the PR.
- **Report to:** the operator, who relays progress to the orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. Otherwise use the named
fallback command above.

Once the worktree checks out, read `AGENTS.md`, contract 022, the
milestone, all three cards, and the Card 230-234 closeouts. Write the
skill from the merged evidence with the code open beside you — every
factual claim in it should be checkable against a closeout or a test.
When you reach a natural pause, tell the operator what changed, what
you actually ran, and what remains.

## Completion Protocol

### Before you start

1. Read this handoff. Its `worker_mode: implementation` and
   `dispatch_authority: orchestrator` metadata activate worker mode.
   Then run one quick read-only probe before broad reads:
   `git rev-parse --show-toplevel`, `git branch --show-current`,
   `git status --porcelain`, `git worktree list --porcelain`.
2. If the current root is a registered worktree with empty status on a
   non-`main` branch, accept it as the launcher-provided worktree.
   Record its actual root/branch; do not create another because it
   differs from the placeholders here.
3. Only if the current context is `main`, dirty, unregistered, or
   unusable: run the named worktree command above (container
   `/Users/tom/Dev/worktrees`, operator-confirmed). Never clean, reset,
   or stash a dirty checkout. If the launcher supplied a dirty or
   `main` worktree, stop and report it.
4. From the selected worktree, after `git fetch origin`, confirm
   `git rev-parse HEAD` equals `git rev-parse origin/main`, confirm
   `git merge-base --is-ancestor 982eff16945137728eab5b5430835a8067b30c5c HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read the milestone, all three cards, `AGENTS.md`, contract 022, and
   the Card 230-234 closeouts.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`)
   and record what you actually ran.

### While you work

- Execute Cards 235, 236, 237 in order. Keep commits aligned with
  meaningful chunks.
- After each meaningful chunk, report through the operator: changed
  files, validation actually run, remaining cards, new risks, blockers.
- Stop and say so if: the skill would need to document behavior the
  surface lacks; the drift check needs core surface beyond the
  pre-authorized read-only export; effigy cannot express the cross-repo
  install; the dogfood pass cannot complete skill-only; or `effigy qa`
  fails in a way that changes the plan.
- Edits outside the pre-authorized seam list: report first, then land
  with evidence.
- Do not quietly turn an open question into new architecture.

### When the assigned runway is complete

1. Run the required final validation: `effigy qa` with the drift check,
   all fixtures, the committed dogfood run; record results honestly.
2. Update all three cards' acceptance boxes and closeouts (actual
   worktree/branch, evidence paths), tick the milestone runway, and
   write the lane log under `docs/logs/2026-08/` with an index line in
   `docs/logs/README.md`.
3. Push the worker branch.
4. Open a PR against `main`. The base above is the planning commit
   before this handoff existed, not a self-referential hash.
5. In the PR body, link the milestone, cards, changed surfaces,
   evidence, validation, and unresolved items.
6. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against contract 022, the cards, and
the diff, independently of your narrative — including reading the skill
as its audience and spot-running the finder and install paths.
Orchestrator and worker share a GitHub identity, so the verdict lands
as a PR comment — that comment is the canonical review record. If
changes are requested, make only those changes on this branch and push
again. The operator alone authorises merge.

- **Closeout refs:** Cards 235-237;
  `docs/roadmaps/g02/033-agent-control-adoption-kit.md`;
  `docs/roadmaps/g02/README.md`; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by
  you).

### Handoff closeout

Before calling the runway complete, leave the cards, milestone, and
evidence state honest. The dogfood card's bar is strict on purpose — a
skill that needed a workaround is not done, it's a finding with the fix
still owed. If something cannot be closed honestly, record it and stop
rather than rounding up.
