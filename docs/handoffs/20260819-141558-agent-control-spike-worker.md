---
title: Agent control spike worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-19
updated: 2026-08-19
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260819-141558-agent-control-spike-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

The planning thread designed an in-app control surface so agents can drive
a running consumer app — screenshots, clicks, typing — without stealing OS
focus or the pointer. It settled the protocol (a stateless MCP server over
streamable HTTP, inside the app, dev builds only), drafted contract 022,
wrote research memo 024, and compiled milestones g02.029-032.

Two questions could not be answered from documentation, and everything
after the spike gates on them. Your job is the spike that answers them:
Card 227, a standalone throwaway prototype under `prototypes/`.

This file is your complete brief; you don't need the planning transcript.

## Why It Matters

Five sibling apps are tested by agents through OS-level computer use
today, which makes the operator's machine unusable while it happens. If
the spike's answers hold, contract 022 gets promoted and three further
milestones build the real thing into Longhorn. If an answer is negative,
that's equally valuable — it corrects the contract before any shared code
exists. A negative result is a successful spike.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `7a8d7ae94dbf14e3da19ceac460c1bbe74b3a1e2`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at that
  commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff file.
- **Worker mode:** implementation worker dispatched by the orchestrator;
  this handoff activates the worker-only worktree preflight.
- **Planning artifacts included at the base:** contract 022 (draft), memo
  024, milestones g02.029-032, Card 227.
- **Worker branch:** `worker/227-agent-control-spike`
- **Worker worktree:** launcher-provided when available; otherwise a unique
  worktree under the operator-configured container (see policy below).
- **Worktree creation command (manual fallback only):**
  `git worktree add <container>/longhorn-227 -b worker/227-agent-control-spike origin/main`
- **Worker worktree policy:** first use the clean, dedicated, non-`main`
  registered worktree supplied by the launcher, even if its generated path
  or branch differs from these placeholders. Record the actual path/branch
  and never create a second worktree for that reason. Only if the current
  context is `main`, dirty, unregistered, or otherwise unusable, read
  `.agents.local.env` and require `AGENTS_WORKTREE_CONTAINER_DIR`. That
  file does not exist right now — if you reach this branch, ask the
  operator for the container directory. Never use `/tmp`, `TMPDIR`, or a
  guessed path.
- **Active spec lane:** research memo 024
  (`docs/research/translation-memos/024-agent-app-control.md`)
- **Roadmap milestone:** g02.029
  (`docs/roadmaps/g02/029-agent-control-spike.md`)
- **Ready cards, in order:** Card 227
  (`docs/roadmaps/g02/batch-cards/227-agent-control-spike.md`)
- **Allowed runway:** Card 227 only.
- **Remaining card budget:** one card; then stop and report.
- **Dispatch topology:** serial — a single worker lane.
- **Parallel safety check:** not applicable; no other lane is dispatched.
- **Canonical refs:** `AGENTS.md`; contract 001 (working rules); contract
  022 draft (`docs/contracts/022-agent-app-control.md`) — evidence target,
  not execution authority; memo 024.
- **Model capability profile:** capable coding model; escalate through the
  operator if the macOS capture probe needs judgment the card doesn't
  cover.
- **Tool/runtime restrictions:** macOS host required (WKWebView probes);
  network access for crates.io.
- **Required validation:** prototype builds and runs; `effigy doctor`
  clean; `effigy qa` only if a shared file changed (and that itself is a
  stop-and-report event).
- **PR base/head:** `main` ← `worker/227-agent-control-spike` (or the
  launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** Card 227 — the prototype under `prototypes/agent-control/`,
  the probe runs, and the evidence updates to memo 024 and the card.
- **Out of scope:** any shared crate or package change; creating
  `longhorn-agent-control` or the Tauri plugin (that's g02.030-031, gated
  on promotion); editing contract 022; native menus/dialogs; anything
  beyond macOS.
- Do not invent architecture, change contracts, widen the roadmap, or
  settle an unresolved product, API, persistence, or security decision.
- Shared checkout discipline from `AGENTS.md` applies: stage by explicit
  path, never `git add -A`, never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout or an unrelated dirty checkout.
- Do not merge the PR. Merge remains a separate operator-authorised action.

## Important Context

- **Planning lineage:** this thread ran protocol research (CDP rejected;
  MCP 2026-07-28 stateless chosen), drafted contract 022, wrote memo 024,
  and compiled g02.029-032. The g02 README's dependency shape shows the
  gate: spike → promotion → core → host → semantic surface.
- **Why this card is ready:** it needs no contract — it's research
  evidence under memo 024, executed as a prototype the way
  `prototypes/gpui-*` and `prototypes/native-content-*` were. The
  governing refs are current at the planning base.
- **Decisions already made (don't relitigate):** stateless MCP over
  streamable HTTP via `rmcp` (`StreamableHttpService` is a Tower service —
  mount it on axum inside the app); capture via
  `Webview::with_webview` → `objc2_web_kit::WKWebView` →
  `takeSnapshot(with:completionHandler:)` (public API, main-thread
  closure); localhost binding with a random port.
- **The freshness trick:** the page renders a counter ticking every
  second, so a screenshot is decidably fresh or stale by reading the
  number in the image. Keep that; it's what makes the probe evidence
  rather than impression.
- **Open tensions:** WebKit may throttle rendering for occluded or
  minimized windows — that's precisely what you're measuring. If occluded
  (not minimized) capture is stale, that's a contract-shaping negative
  result: record it in memo 024 and stop rather than hunting for private
  workarounds.
- **Client probe:** any current MCP client works; Claude Code's
  `claude mcp add --transport http` against your localhost port is the
  easy path. Record the negotiated protocol revision and whether any
  session id appears.
- **Report after:** (1) prototype builds and both tools answer a real MCP
  client; (2) probe matrix complete and memo 024 updated; then the PR.
- **Report to:** the operator, who relays progress to the orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. If it's `main`, dirty, or
unregistered, follow the fallback chain in the policy above — and since
`.agents.local.env` doesn't exist yet, that path ends at asking the
operator, not guessing.

Once the worktree checks out, read `AGENTS.md`, Card 227, milestone
g02.029, memo 024, and contract 022's draft from the worker worktree, then
start with the smallest thing that can fail: a bare Tauri window with the
ticking counter, then the rmcp mount, then the two tools, then the probe
matrix. When you reach a natural pause, tell the operator what changed,
what you actually ran, and what remains.

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
   unusable: follow the manual fallback in the worktree policy above,
   asking the operator for the container directory (the config file is
   currently absent). Never clean, reset, or stash a dirty checkout.
4. From the selected worktree, confirm `git rev-parse HEAD` equals
   `git rev-parse origin/main` after `git fetch origin`, confirm
   `git merge-base --is-ancestor 7a8d7ae94dbf14e3da19ceac460c1bbe74b3a1e2 HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read Card 227, milestone g02.029, `AGENTS.md`, and the canonical refs.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`) and
   record what you actually ran.

### While you work

- Execute Card 227 only. Keep commits aligned with meaningful chunks.
- After each meaningful chunk, report through the operator: changed files,
  validation actually run, what remains, new risks, blockers.
- Stop and say so if the spike starts wanting shared-crate changes, a
  probe needs private API or a permission prompt, intent is ambiguous, or
  a result contradicts the plan. A negative probe result is a stop-and-
  record, not a failure to fix.
- Do not quietly turn an open question into new architecture.

### When the runway is complete

1. Final validation: prototype builds and runs; probe matrix recorded;
   `effigy doctor` clean; `effigy qa` only if any shared file changed.
2. Update memo 024 (gaps → findings with evidence) and Card 227's
   acceptance boxes and closeout notes, including actual crate versions
   and the actual worktree/branch used.
3. Push the worker branch.
4. Open a PR against `main`. The base above is the planning commit before
   this handoff existed, not the handoff commit itself.
5. In the PR body, link Card 227, milestone g02.029, memo 024, the changed
   files, the probe evidence, and anything unresolved.
6. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against Card 227, memo 024, and contract
022's draft, independently of your narrative. Orchestrator and worker
share a GitHub identity, so the verdict lands as a PR comment — that
comment is the canonical review record. If changes are requested, make
only those changes on this branch and push again. The operator alone
authorises merge.

- **Closeout refs:** Card 227; `docs/roadmaps/g02/029-agent-control-spike.md`;
  `docs/roadmaps/g02/README.md`; memo 024; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by you).

### Handoff closeout

Before calling the runway complete, leave the card, milestone, and memo
state honest. If a probe is blocked, record the blocker and stop rather
than making the spike look more complete than it is.
