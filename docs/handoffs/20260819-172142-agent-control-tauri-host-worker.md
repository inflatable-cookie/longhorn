---
title: Agent control Tauri host worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-19
updated: 2026-08-19
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260819-172142-agent-control-tauri-host-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

The agent app-control runway is two lanes in: the spike (Card 227, PR 2)
proved the mechanics on the wire, and the core crate
`longhorn-agent-control` (Cards 228-229, PR 3) landed the stateless MCP
server, discovery, token, and tool vocabulary, host-free. Your run is the
host half: milestone g02.031, Cards 230 and 231 — the Tauri plugin that
composes the server inside a real app, and the capture path proved against
a packaged `.app`.

This file is your complete brief; you don't need the planning transcript.

## Why It Matters

This is the lane where the feature first exists inside an actual
application. It also carries the two claims consumers will lean on
hardest: that release builds contain none of this surface, and that
screenshots of an unfocused, occluded, or minimized window are fresh.
Both get proved here, not asserted — the first by a repeatable artifact
scan, the second by re-running the spike's freshness matrix against a
packaged bundle, where hardened-runtime or sandbox differences would
show up.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `a72f90d72e69ecd32c6f0a4ce004b03d3d0bd940`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at that
  commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff file.
- **Worker mode:** implementation worker dispatched by the orchestrator;
  this handoff activates the worker-only worktree preflight.
- **Planning artifacts included at the base:** contract 022 (active,
  Origin wording aligned to the Card 229 guard), merged
  `crates/longhorn-agent-control/`, milestone g02.031 (ready), Cards
  230-231, the Card 227 spike and its evidence.
- **Worker branch:** `worker/231-agent-control-tauri-host` — the branch
  number is a label; the cards are 230 and 231.
- **Worker worktree:** launcher-provided when available; otherwise a
  unique worktree under `/Users/tom/Dev/worktrees` (operator-confirmed
  container).
- **Worktree creation command (manual fallback only):**
  `git worktree add /Users/tom/Dev/worktrees/longhorn-231 -b worker/231-agent-control-tauri-host origin/main`
- **Worker worktree policy:** first use the clean, dedicated, non-`main`
  registered worktree supplied by the launcher, even if its generated path
  or branch differs from these placeholders. Record the actual path/branch
  and never create a second worktree for that reason. Only if the current
  context is `main`, dirty, unregistered, or otherwise unusable, use the
  named command above. `.agents.local.env` still does not exist (PAPERCUTS
  records why); the container above is the operator's standing answer.
  Never use `/tmp`, `TMPDIR`, or a guessed path.
- **Active spec lane:** none open — contract 022 and memo 024 are
  promoted background.
- **Roadmap milestone:** g02.031
  (`docs/roadmaps/g02/031-agent-control-tauri-host.md`)
- **Ready cards, in order:**
  `docs/roadmaps/g02/batch-cards/230-agent-control-tauri-plugin.md`, then
  `docs/roadmaps/g02/batch-cards/231-agent-control-capture.md`
- **Allowed runway:** Cards 230 and 231, one PR.
- **Remaining card budget:** two cards; then stop and report.
- **Dispatch topology:** serial — 231 captures through 230's plugin and
  window scope.
- **Parallel safety check:** not applicable; one lane.
- **Canonical refs:** `AGENTS.md`; contract 022
  (`docs/contracts/022-agent-app-control.md`); contracts 001, 006
  (command registry the plugin invokes through), 010 (IPC boundary), 012
  (workspace admission), 020 (host adapter rules; per-backend evidence
  discipline); memo 024 and the Card 227/229 closeouts for mechanics.
- **Model capability profile:** capable coding model; the release-absence
  scan and objc2 capture bridge deserve full care.
- **Tool/runtime restrictions:** macOS host required — Card 231 packages
  an app and runs the freshness matrix on a real display. Network for
  crates.io.
- **Required validation:** `effigy qa`; the Card 230 artifact scan run
  feature-on and feature-off; Card 231's packaged freshness matrix;
  `effigy doctor` for orientation.
- **PR base/head:** `main` ← `worker/231-agent-control-tauri-host` (or
  the launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** the new `longhorn-tauri-agent-control` crate (Card 230),
  the capture path and packaged freshness proof (Card 231), the
  workspace/docs wiring contract 012 requires, and whatever minimal proof
  composition Card 231 needs (extend an existing `examples/` proof app or
  add one).
- **Out of scope:** the TS shim and semantic tools — `snapshot`, input
  dispatch, `evaluate`, `wait_for` implementations are g02.032; here they
  answer the core vocabulary's typed `Unsupported`. Also out: consumer
  repos, contract edits, the core crate's public surface (extend only if
  a card genuinely requires it — that's a report first), and the spike
  prototype (frozen evidence).
- The spike's `control.rs` is donor evidence for Card 231's mechanics
  (main-thread dispatch, retained webview, completion → oneshot) — reuse
  the proven shape, not the spike's `expect()`s or its stdout discovery.
- Version intel, not instruction: the spike pinned tauri-runtime 2.10.1
  under tauri 2.10.3. The workspace's existing tauri pins govern; if they
  conflict with what the plugin needs, stop and report rather than moving
  a workspace pin unilaterally (contract 012 treats held-back pins as
  decisions, not chores).
- Do not invent architecture, change contracts, widen the roadmap, or
  settle an unresolved product/API/persistence/security decision.
- Shared checkout discipline: stage by explicit path, never `git add -A`,
  never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout.
- Do not merge the PR. Merge remains a separate operator-authorised
  action.

## Important Context

- **Planning lineage:** memo 024 → contract 022 → g02.029 spike (PR 2) →
  g02.030 core (PR 3) → this lane. `docs/logs/2026-08/` has both prior
  closeouts; the g02 README dependency shape shows 031 → 032.
- **Why these cards are ready:** the core crate's `ControlHandler` seam
  and server assembly are merged and conformance-proved; the capture
  mechanics are spike-proved; contract 022 is active. Nothing here waits
  on an unmade decision.
- **Decisions already made (don't relitigate):** dev-only gating is
  compile-time and total — no runtime toggle; feature-off release
  artifacts must be scan-provably clean, and feature unification pulling
  server code into a release build is a stop condition, not a foot-note;
  unwired tools answer typed `Unsupported`, never stub success; command
  invocation goes through the contract 006 registry, adding no authority;
  capture is public API only.
- **The one result that would change the plan:** if the packaged app's
  freshness matrix diverges from the spike's debug-binary results (stale
  occluded snapshots under hardened runtime, a permission prompt, an
  entitlement need), stop, record the matrix, and bring it back — that
  amends contract 022 before g02.032 builds on it. A negative there is a
  successful card, exactly like the spike.
- **Freshness matrix logistics:** the spike scripted its window states
  (occlusion via a covering window, minimize/restore) and judged
  freshness by `evaluate`-bracketing a DOM counter — reuse that method.
  It needs the operator's display for a few minutes; coordinate the run
  through the operator rather than assuming the desktop is free.
- **Open tensions:** none recorded beyond the packaged-behavior question
  above. The Card 229 closeout's `effigy qa` bootstrap notes (bun link,
  `POODLE_REPO`) will likely apply to your worktree too.
- **Report after:** (1) Card 230 composed and both artifact-scan
  directions green; (2) Card 231's packaged matrix recorded; then the PR.
- **Report to:** the operator, who relays progress to the orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. Otherwise use the named
fallback command above.

Once the worktree checks out, read `AGENTS.md`, contract 022, milestone
g02.031, both cards, and the core crate's `handler` and `server` modules.
Start with Card 230's plugin skeleton and feature gating — the artifact
scan is worth building early, because it shapes how the gating must be
structured — then the handler's window scope, then Card 231 on top. When
you reach a natural pause, tell the operator what changed, what you
actually ran, and what remains.

## Completion Protocol

### Before you start

1. Read this handoff. Its `worker_mode: implementation` and
   `dispatch_authority: orchestrator` metadata activate worker mode. Then
   run one quick read-only probe before broad reads:
   `git rev-parse --show-toplevel`, `git branch --show-current`,
   `git status --porcelain`, `git worktree list --porcelain`.
2. If the current root is a registered worktree with empty status on a
   non-`main` branch, accept it as the launcher-provided worktree. Record
   its actual root/branch; do not create another because it differs from
   the placeholders here.
3. Only if the current context is `main`, dirty, unregistered, or
   unusable: run the named worktree command above (container
   `/Users/tom/Dev/worktrees`, operator-confirmed). Never clean, reset,
   or stash a dirty checkout. If the launcher supplied a dirty or `main`
   worktree, stop and report it.
4. From the selected worktree, after `git fetch origin`, confirm
   `git rev-parse HEAD` equals `git rev-parse origin/main`, confirm
   `git merge-base --is-ancestor a72f90d72e69ecd32c6f0a4ce004b03d3d0bd940 HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read both cards, milestone g02.031, `AGENTS.md`, contract 022, and the
   core crate's handler/server seams.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`) and
   record what you actually ran.

### While you work

- Execute Card 230 then Card 231. Keep commits aligned with meaningful
  chunks.
- After each meaningful chunk, report through the operator: changed
  files, validation actually run, remaining cards, new risks, blockers.
- Stop and say so if compile-time gating cannot fully exclude the surface
  from a featureless build, command invocation would need new authority,
  a workspace version pin blocks the plugin, packaged capture diverges
  from the spike evidence, or `effigy qa` fails in a way that changes the
  plan.
- Do not quietly turn an open question into new architecture.

### When the assigned runway is complete

1. Run the required final validation: `effigy qa`, the artifact scan both
   ways, and the packaged freshness matrix; record results honestly.
2. Update both cards' acceptance boxes and closeout notes (including the
   matrix evidence, resolved crate versions, and the actual
   worktree/branch used) and tick the milestone runway boxes.
3. Push the worker branch.
4. Open a PR against `main`. The base above is the planning commit before
   this handoff existed, not a self-referential hash.
5. In the PR body, link the milestone, both cards, contract 022, the
   changed surfaces, scan and matrix evidence, validation, and unresolved
   items.
6. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against contract 022, both cards, and the
diff, independently of your narrative. Orchestrator and worker share a
GitHub identity, so the verdict lands as a PR comment — that comment is
the canonical review record. If changes are requested, make only those
changes on this branch and push again. The operator alone authorises
merge.

- **Closeout refs:** Cards 230-231;
  `docs/roadmaps/g02/031-agent-control-tauri-host.md`;
  `docs/roadmaps/g02/README.md`; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by
  you).

### Handoff closeout

Before calling the runway complete, leave the cards, milestone, and
evidence state honest. If a card is blocked — especially on packaged
capture behavior — record the blocker and the partial matrix and stop
rather than making the run look more complete than it is.
