---
title: Agent control core worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-19
updated: 2026-08-19
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260819-154752-agent-control-core-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

The agent app-control lane is building an in-app control surface so agents
can drive a running consumer app without stealing OS focus or the pointer.
The spike (Card 227, PR 2) proved both risky assumptions on the wire, and
contract 022 plus memo 024 are now promoted. Your run is the first real
implementation slice: milestone g02.030, Cards 228 and 229, which create
`longhorn-agent-control` — the host-agnostic core crate — and prove its
stateless server without any host.

This file is your complete brief; you don't need the planning transcript.

## Why It Matters

Five sibling apps will compose this. The core crate is where the contract's
vocabulary becomes types, so decisions that look small here (what a ref is,
what `wait_for` can express, where discovery files live) are what every
host and every consumer inherit. The spike already paid for the risky
unknowns — this run is about getting the boundary right, not about
discovery.

## Current State

Here is the state you're inheriting:

- **Repository:** `git@github.com:inflatable-cookie/longhorn.git`
- **Planning branch:** `main`
- **Planning base commit:** `458626d35172672d121d870ccbf7594fd20982ff`
- **Pushed main verification:** local `HEAD` equalled `origin/main` at that
  commit when this handoff was written.
- **Planning checkout:** clean apart from this handoff and a PAPERCUTS
  entry committed with it.
- **Worker mode:** implementation worker dispatched by the orchestrator;
  this handoff activates the worker-only worktree preflight.
- **Planning artifacts included at the base:** contract 022 (active), memo
  024 (promoted), milestone g02.030 (ready), Cards 228-229, the Card 227
  spike under `prototypes/agent-control/`.
- **Worker branch:** `worker/230-agent-control-core` — the branch number is
  just a label; the cards are 228 and 229.
- **Worker worktree:** launcher-provided when available; otherwise a unique
  worktree under `/Users/tom/Dev/worktrees` (operator-confirmed container
  from the previous run).
- **Worktree creation command (manual fallback only):**
  `git worktree add /Users/tom/Dev/worktrees/longhorn-230 -b worker/230-agent-control-core origin/main`
- **Worker worktree policy:** first use the clean, dedicated, non-`main`
  registered worktree supplied by the launcher, even if its generated path
  or branch differs from these placeholders. Record the actual path/branch
  and never create a second worktree for that reason. Only if the current
  context is `main`, dirty, unregistered, or otherwise unusable, use the
  named command above. `.agents.local.env` still does not exist (a
  PAPERCUTS entry records why); the container above is the operator's
  standing answer. Never use `/tmp`, `TMPDIR`, or a guessed path.
- **Active spec lane:** memo 024 (promoted; background, not open questions)
- **Roadmap milestone:** g02.030
  (`docs/roadmaps/g02/030-agent-control-core.md`)
- **Ready cards, in order:**
  `docs/roadmaps/g02/batch-cards/228-agent-control-core-crate.md`, then
  `docs/roadmaps/g02/batch-cards/229-agent-control-stateless-server.md`
- **Allowed runway:** Cards 228 and 229, one PR.
- **Remaining card budget:** two cards; then stop and report.
- **Dispatch topology:** serial — 229 builds directly on 228's types.
- **Parallel safety check:** not applicable; one lane.
- **Canonical refs:** `AGENTS.md`; contract 022
  (`docs/contracts/022-agent-app-control.md`) — now execution authority;
  contracts 001 (working rules), 004 (storage roots, for discovery paths),
  006 (command vocabulary), 012 (workspace admission); memo 024 for
  rationale.
- **Model capability profile:** capable coding model; the security-shaped
  pieces (token handling, auth-before-dispatch ordering) deserve your
  full care — escalate through the operator if contract 022 underdetermines
  one of them.
- **Tool/runtime restrictions:** network for crates.io; no macOS-specific
  code belongs in this crate at all (host-free is an acceptance criterion).
- **Required validation:** `effigy qa` — this run changes the shared
  workspace, unlike the spike. `effigy doctor` for orientation.
- **PR base/head:** `main` ← `worker/230-agent-control-core` (or the
  launcher-provided branch you recorded).
- **PR URL:** pending
- **Review state:** awaiting-review once opened
- **Merge authorisation:** operator only; never merge.

## Boundaries

Please keep this run inside the named runway:

- **In scope:** the new `longhorn-agent-control` crate (Card 228) and its
  stateless server assembly with conformance fixtures (Card 229), plus the
  workspace wiring contract 012 requires (workspace member, deny/MSRV
  gates, card/milestone evidence updates).
- **Out of scope:** the Tauri plugin, capture, dev-feature gating, and
  release-absence proof (g02.031); the TS shim and semantic tools
  (g02.032); any consumer composition; any edit to contract 022; any
  change to the spike prototype.
- The g02 "no crate add/remove" guardrail scopes to remediation work only —
  this milestone adds a crate deliberately, like g02.009/010 did. The
  nucleus boundary verifier note in the README is about consumer repos,
  which you don't touch.
- The spike code in `prototypes/agent-control/` is donor evidence, not the
  shared contract (AGENTS.md rule). Reuse its proven configuration facts
  (rmcp 3.x, `legacy_session_mode: false`); don't copy its shortcuts —
  `expect()`-driven serving, stdout discovery, and unauthenticated tools
  were spike-acceptable and are not crate-acceptable.
- Do not invent architecture, change contracts, widen the roadmap, or
  settle an unresolved product/API/persistence/security decision.
- Shared checkout discipline: stage by explicit path, never `git add -A`,
  never stash.
- Work only in the selected clean worker worktree. Never edit the
  orchestrator's planning checkout.
- Do not merge the PR. Merge remains a separate operator-authorised action.

## Important Context

- **Planning lineage:** memo 024 → contract 022 → g02.029 spike (PR 2,
  merged 2026-08-19) → this milestone. The g02 README's dependency shape
  shows 030 → 031 → 032. The log entry
  `docs/logs/2026-08/19-agent-control-spike-and-promotion.md` is the short
  version of everything that led here.
- **Why these cards are ready:** contract 022 is active; both runtime
  unknowns are closed with wire evidence; the cards' scope is bounded to
  the host-free core, so nothing in them waits on macOS behavior.
- **Decisions already made (don't relitigate):** stateless only — no
  session ids ever, no server-side ref table; refs are opaque and resolved
  by the edge that stamped them; `wait_for` has no time-only or rAF form
  (contract 022 records why — WKWebView stops rAF while not key); tokens
  are credentials (constant-time compare, never logged, never in Debug);
  discovery follows contract 004 root conventions rather than a hand-rolled
  dirs lookup.
- **Spike intel you'll want:** rmcp 3.1.3 supports 2026-07-28 but defaults
  to 2025-11-25 — Card 229 asks you to confirm or correct that for the
  version you pin, in the card closeout. GET/DELETE answer 405 in
  stateless mode by construction; your fixtures should pin that too.
- **Open tensions:** rmcp's middleware surface for auth-before-dispatch
  ordering is unverified — Card 229 names it as a stop condition if it
  can't be expressed cleanly. The provider seam (Card 228) should be the
  smallest trait that lets a native surface register later; if you find it
  wants to grow, that's a report, not a design session.
- **Report after:** (1) Card 228's crate compiles host-free with its
  fixtures green; (2) Card 229's conformance suite green under
  `effigy qa`; then the PR.
- **Report to:** the operator, who relays progress to the orchestrator.

## Suggested Next Move

This handoff explicitly activates worker mode. Before broad repository
reads, run the quick startup worktree-safety preflight in
`## Completion Protocol`. If the current context is a clean, dedicated,
non-`main` registered worktree, it's the launcher-provided one: use it,
record its actual path/branch, and move on. Otherwise use the named
fallback command above.

Once the worktree checks out, read `AGENTS.md`, contract 022, milestone
g02.030, and both cards from the worker worktree. Start with Card 228's
tool vocabulary — it's the piece everything else types against — then
discovery and token, then the seam, then Card 229 on top. When you reach a
natural pause, tell the operator what changed, what you actually ran, and
what remains.

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
   `/Users/tom/Dev/worktrees`, operator-confirmed). Never clean, reset, or
   stash a dirty checkout. If the launcher supplied a dirty or `main`
   worktree, stop and report it.
4. From the selected worktree, after `git fetch origin`, confirm
   `git rev-parse HEAD` equals `git rev-parse origin/main`, confirm
   `git merge-base --is-ancestor 458626d35172672d121d870ccbf7594fd20982ff HEAD`
   succeeds, and confirm this handoff file exists in `HEAD`.
5. Read both cards, milestone g02.030, `AGENTS.md`, and contract 022.
6. Run the repo's cheap orientation (`effigy tasks`, `effigy doctor`) and
   record what you actually ran.

### While you work

- Execute Card 228 then Card 229. Keep commits aligned with meaningful
  chunks.
- After each meaningful chunk, report through the operator: changed files,
  validation actually run, remaining cards, new risks, blockers.
- Stop and say so if contract 022 underdetermines a security decision, the
  auth-before-dispatch ordering can't be expressed in rmcp, statelessness
  would require per-client state, the provider seam wants to grow, or
  `effigy qa` fails in a way that changes the plan.
- Do not quietly turn an open question into new architecture.

### When the assigned runway is complete

1. Run the required final validation: `effigy qa`, and record the result
   honestly.
2. Update both cards' acceptance boxes and closeout notes (including the
   exact rmcp version and its negotiated-revision behavior, and the actual
   worktree/branch used) and tick the milestone runway boxes.
3. Push the worker branch.
4. Open a PR against `main`. The base above is the planning commit before
   this handoff existed, not a self-referential hash.
5. In the PR body, link the milestone, both cards, contract 022, the
   changed surfaces, fixture evidence, validation, and unresolved items.
6. Report the PR URL and evidence to the operator. Do not merge.

### Review and merge path

The orchestrator reviews the PR against contract 022, both cards, and the
diff, independently of your narrative. Orchestrator and worker share a
GitHub identity, so the verdict lands as a PR comment — that comment is
the canonical review record. If changes are requested, make only those
changes on this branch and push again. The operator alone authorises
merge.

- **Closeout refs:** Cards 228-229;
  `docs/roadmaps/g02/030-agent-control-core.md`;
  `docs/roadmaps/g02/README.md`; `docs/roadmaps/generation-index.md`
  (Next Task pointer is updated by the orchestrator at merge, not by you).

### Handoff closeout

Before calling the runway complete, leave the cards, milestone, and
evidence state honest. If a card is blocked, record the blocker and stop
rather than making the run look more complete than it is.
