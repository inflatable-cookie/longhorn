---
title: Agent control screenshot composition worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-20
updated: 2026-08-20
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260820-143033-agent-control-composition-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

Figmatic's adoption of the agent-control surface (its PR 14) surfaced
the runway's first mechanism-vs-contract gap: contract 022 promises
that native-content islands appear in screenshots, but `screenshot`
snapshots only the window's same-label UI webview, so Figmatic's
attached preview child webview comes back as a black rectangle. The
finding arrived as a full handoff from the Figmatic thread —
`docs/handoffs/20260820-142528-figmatic-child-webview-screenshot-composition.md`
— and it is this lane's primary record: read it in full before code.

Your run is milestone g02.034, Card 238: make the capture mechanism
match the contract's claim, or bring back the evidence that no
public-API mechanism can.

This file plus the Figmatic handoff are your complete brief.

## Why It Matters

Figmatic's core working surface *is* the preview island. An agent that
can drive the shell but cannot see the preview cannot verify the thing
the app exists to produce. The consumer is relying on exactly the
boundary contract 022 drew — screenshot visibility without semantic
control — and right now the contract overclaims what the mechanism
delivers. Fixing truthfully unblocks Figmatic PR 14 and every future
multiwebview consumer.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `912f57c48c08f12438b84b2505eee9daf4ee2248`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at
  that commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff file.
- **Worker mode:** implementation worker dispatched by the
  orchestrator; this handoff activates the worker-only worktree
  preflight.
- **Planning artifacts included at the base:** the Figmatic handoff,
  milestone g02.034, Card 238, and the `c1482daf` multiwebview
  enumeration fix with its mount fixture.
- **Worker branch:** `worker/238-agent-control-composition` — a label;
  the card is 238.
- **Worker worktree:** launcher-provided when available; otherwise a
  unique worktree under `/Users/tom/Dev/worktrees` (operator-confirmed
  container).
- **Worktree creation command (manual fallback only):**
  `git worktree add /Users/tom/Dev/worktrees/longhorn-238 -b worker/238-agent-control-composition origin/main`
- **Worker worktree policy:** first use the clean, dedicated,
  non-`main` registered worktree supplied by the launcher, even if its
  generated path or branch differs from these placeholders. Record the
  actual path/branch and never create a second worktree for that
  reason. Only if the current context is `main`, dirty, unregistered,
  or otherwise unusable, use the named command above.
  `.agents.local.env` still does not exist (PAPERCUTS records why); the
  container above is the operator's standing answer. Never use `/tmp`,
  `TMPDIR`, or a guessed path.
- **Active spec lane:** none — this executes under contract 022, whose
  screenshot claim the card makes true or narrows.
- **Roadmap milestone:** g02.034
  (`docs/roadmaps/g02/034-agent-control-window-composition.md`)
- **Ready cards, in order:**
  `docs/roadmaps/g02/batch-cards/238-agent-control-screenshot-composition.md`
- **Allowed runway:** Card 238 only, one PR.
- **Remaining card budget:** one card; then stop and report.
- **Dispatch topology:** serial — a single lane.
- **Parallel safety check:** not applicable.
- **Canonical refs:** `AGENTS.md`; contract 022; the Figmatic handoff
  (its Boundaries and Suggested Next Move sections are requirements,
  not suggestions); contracts 001, 012, 020; the Card 226 closeout
  (the read-the-platform-before-building lesson); the Card 231/234
  closeouts (capture mechanics, packaged proof pattern).
- **Expected seam-touches, pre-authorized:** `capture.rs`,
  `handler.rs`, `mount.rs` in `longhorn-tauri-agent-control`;
  `examples/agent-control-proof` (child webview, drivers, evidence);
  contract 022's screenshot/`wait_for` wording and the composition
  guide's child-webview section as the truthfulness pass; the plugin's
  test files. Anything else — core crate vocabulary, new dependencies
  beyond what capture already uses, effigy wiring — is report-first
  through the operator before the edit.
- **Model capability profile:** capable coding model; the geometry
  (bounds, scale, clipping, z-order) and the objc2 bridge deserve full
  care.
- **Tool/runtime restrictions:** macOS host; packaged proofs run on
  the operator's display without focus or pointer theft.
- **Required validation:** `effigy qa`;
  `check:agent-control-release-absence` explicitly if qa does not
  surface it; the packaged proof matrix with the child attached.
- **PR base/head:** `main` ← `worker/238-agent-control-composition`
  (or the launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** whole-window screenshot composition per Card 238; the
  extended proof app; regression fixtures; packaged evidence; the
  contract/guide truthfulness pass; the Figmatic rerun handback.
- **Out of scope:** semantic tools inside child webviews (snapshot,
  click, type, `wait_for` stay UI-webview-only — the Figmatic handoff
  explicitly does not ask for them); Figmatic-side workarounds; any
  consumer repo edit; screen-recording permission, private API, or OS
  desktop capture (a finding that they're required is a stop, not a
  route); weakening dev-only gating or release absence.
- The black island is not redefinable as acceptable output — the
  Figmatic handoff forbids it in as many words.
- Composition policy stays consumer-neutral: compose what the window
  hosts; no app-specific visibility rules enter Longhorn.
- Shared checkout discipline: stage by explicit path, never
  `git add -A`, never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout.
- Do not merge the PR. Merge remains a separate operator-authorised
  action.

## Important Context

- **Sequence discipline:** reproduce the black island first (baseline
  PNG committed), characterize the supported capture mechanisms second,
  implement third. Card 226's closeout is in the governing refs because
  its lesson — evidence partway through can change the premise; stop
  and say so rather than building on — is the one this lane is most
  likely to need.
- **The leading hypothesis, not the design:** per-webview
  `takeSnapshot` composed by each child's bounds
  (`Webview::position`/`size`), clipped to the parent viewport, in a
  deterministic order. `image` or raw `NSBitmapImageRep` compositing
  both plausible; whatever you choose, the invariants in Card 238 are
  the acceptance, and 1x/2x scale is where naive compositing usually
  lies.
- **Freshness with children:** each webview renders in its own
  process, so per-webview freshness should hold in every window state —
  but that's Card 227's claim re-tested per webview, not assumed. If a
  child's snapshot goes stale in a state where the parent's does not
  (e.g. minimized), that is a recorded narrowing for the contract, not
  a silent hole.
- **Proof method continuity:** the proof app's hue-ticker judgment
  extends naturally — give the child its own hue stride and bracket
  each webview's counter through `evaluate`... noting `evaluate` runs
  in the UI webview only, so the child's bracket needs its own path
  (the mount's shim-eval loop reaches every webview; a child-side
  counter readable via its own eval, or encode the child's counter in
  pixels alone and judge from the PNG with wider tolerance). Solving
  that honestly is part of the card.
- **Known plumbing:** the plugin already walks `Window` +
  `window.webviews()` everywhere (`c1482daf`); tauri's `unstable`
  feature is already on behind `dev`; the mock runtime supports
  `add_child` (see `multiwebview_window_stays_enumerable`) but cannot
  render — pixel proofs live in the packaged driver, like Cards
  231/234.
- **Handback:** Figmatic owns its dependency bump, PR 14 rerun, and
  merge. Your closeout names the exact revision and rerun steps;
  completion (per the Figmatic handoff) is a new Figmatic MCP
  screenshot whose preview island contains rendered content.
- **Report after:** (1) baseline reproduced and mechanisms
  characterized — this is the checkpoint where a premise change would
  surface, report before freezing the design; (2) fixture and
  composition green locally; (3) packaged matrix with the child
  attached; then the PR.
- **Report to:** the operator, who relays progress to the
  orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. Otherwise use the named
fallback command above.

Once the worktree checks out, read the Figmatic handoff end to end,
then Card 238, contract 022, and the capture/handler/mount sources.
Build the failing thing first: the proof app with a visible child and
the driver run that saves the black-island baseline. When you reach a
natural pause, tell the operator what changed, what you actually ran,
and what remains.

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
   `/Users/tom/Dev/worktrees`, operator-confirmed). Never clean,
   reset, or stash a dirty checkout. If the launcher supplied a dirty
   or `main` worktree, stop and report it.
4. From the selected worktree, after `git fetch origin`, confirm
   `git rev-parse HEAD` equals `git rev-parse origin/main`, confirm
   `git merge-base --is-ancestor 912f57c48c08f12438b84b2505eee9daf4ee2248 HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read the Figmatic handoff, Card 238, milestone g02.034,
   `AGENTS.md`, and contract 022.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`)
   and record what you actually ran.

### While you work

- Execute Card 238. Keep commits aligned with meaningful chunks:
  baseline, mechanism characterization, composition, proofs,
  truthfulness pass.
- After each meaningful chunk, report through the operator: changed
  files, validation actually run, what remains, new risks, blockers.
- Stop and say so if: no public-API route composes the image without
  permission prompts, private API, or focus theft (bring the failing
  fixture and the design choices — that is an operator contract
  decision); the mechanism characterization changes the premise;
  composition wants app-specific policy; or `effigy qa` fails in a way
  that changes the plan.
- Edits outside the pre-authorized seam list: report first, then land
  with evidence.
- Do not quietly turn an open question into new architecture.

### When the assigned runway is complete

1. Final validation: `effigy qa`; the release-absence scan explicitly
   if qa does not surface it; the packaged matrix with the child
   attached, every window state.
2. Update Card 238's acceptance boxes and closeout (mechanisms
   considered, chosen design, narrowings if any, exact revision and
   Figmatic rerun steps, actual worktree/branch), tick the milestone,
   apply the contract 022 / guide truthfulness pass, and write a dated
   log under `docs/logs/2026-08/` with an index line.
3. Push the worker branch.
4. Open a PR against `main`. The base above is the planning commit
   before this handoff existed, not a self-referential hash.
5. In the PR body, link the milestone, the card, both handoffs, the
   contract diff, evidence (baseline and fixed PNGs), validation, and
   unresolved items.
6. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against contract 022, Card 238, the
Figmatic handoff's boundaries, and the diff, independently of your
narrative — including reading the before/after PNGs directly.
Orchestrator and worker share a GitHub identity, so the verdict lands
as a PR comment — that comment is the canonical review record. If
changes are requested, make only those changes on this branch and push
again. The operator alone authorises merge.

- **Closeout refs:** Card 238;
  `docs/roadmaps/g02/034-agent-control-window-composition.md`;
  `docs/roadmaps/g02/README.md`; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by
  you).

### Handoff closeout

Before calling the runway complete, leave the card, milestone,
contract, and evidence state honest. If composition works in most
window states but not all, the contract records exactly which — a
narrowed truth beats a smoothed claim, and Figmatic's rerun will test
whatever you wrote.
