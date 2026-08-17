---
title: Consumer-scoped credential slots worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
handoff: single-file-path-only
status: complete
owner: orchestrator
created: 2026-08-16
updated: 2026-08-17
completed: 2026-08-17
outcome: >-
  Card 225 landed. Worker delivered consumer-scoped slots; merged to main as
  PR #1 (7f30368c, feature commit fe5ab2bc). g02.028 is complete. This file is
  history, not a dispatch — do not relaunch a worker from it.
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260816-192517-scoped-credentials-card225.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr]
---

## What This Thread Was Doing

The orchestrator characterized Bovine publishing's secure-store blocker
against Longhorn's realized credential boundary. This worker owns the one
shared-mechanism change: replace the closed slot enum with a validated scoped
value while keeping existing persisted names exact.

## Why It Matters

Bovine needs signing, submit, and status credentials isolated per registered
source. Product-specific enum variants would leak Bovine policy into Longhorn;
a parallel store would duplicate the security boundary.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `ae8852e725acefc96847579b4b2035113914fbbb`
- **Pushed main verification:** `origin/main` equals the planning base before this handoff commit
- **Planning checkout:** clean after the planning commit
- **Planning artifacts included at the base:** `ae8852e7 docs(credentials): compile scoped slot runway`
- **Worker branch:** `codex/scoped-credentials-card225`
- **Worker worktree:** `/Users/tom/Dev/projects/longhorn-worktrees/scoped-credentials-card225`
- **Worktree creation command:** `git worktree add -b codex/scoped-credentials-card225 /Users/tom/Dev/projects/longhorn-worktrees/scoped-credentials-card225 origin/main`
- **Active spec lane:** `docs/contracts/021-consumer-scoped-credential-slots.md`
- **Roadmap milestone:** `docs/roadmaps/g02/028-consumer-scoped-credential-slots.md`
- **Ready cards, in order:** `docs/roadmaps/g02/batch-cards/225-consumer-scoped-credential-slot-value.md`
- **Allowed runway:** Card 225 only
- **Remaining card budget:** one card
- **Canonical refs:** contracts 001, 003, 004, 012, and 021; `crates/longhorn-core/src/credential.rs`; `crates/longhorn-credential-keyring/src/lib.rs`
- **Model capability profile:** frontier coding model, high reasoning
- **Tool/runtime restrictions:** use Effigy; pre-1.0 clean break, no compatibility aliases or product vocabulary
- **Required validation:** focused core/keyring tests, reference and package checks, `effigy qa`
- **PR base/head:** `main` <- `codex/scoped-credentials-card225`
- **PR URL:** pending
- **Review state:** awaiting implementation and orchestrator review
- **Merge authorisation:** not granted; do not merge

## Boundaries

- **In scope:** contract-021 slot value/error, borrowed store API, migration of all Longhorn callers/backends/tests/docs, and exact conformance evidence.
- **Out of scope:** Bovine/Farmyard settings or provisioning, external consumer mutation, enumeration, conditional write, rotation, or new secure-store backend.
- Do not invent architecture, change contracts, widen the roadmap, or choose an unresolved decision.
- Work only in `/Users/tom/Dev/projects/longhorn-worktrees/scoped-credentials-card225` on `codex/scoped-credentials-card225`.
- Do not merge the PR.

## Important Context

- **Planning lineage:** Card 210 moved credential plumbing to `longhorn-core`; Card 224 documented why conditional write is unavailable; Bovine then exposed the remaining closed-vocabulary gap.
- **Why these cards are ready:** exact canonical format, segment bounds, built-in compatibility, store borrowing, exclusions, and stop conditions are frozen in contract 021.
- **Decisions and preferences:** preserve `refresh-token`, `licence-key`, and `backup-identity` byte-for-byte; consumer meaning stays opaque; no alias or dual read.
- **Open tensions:** platform keyring account-name acceptance must be proved, not assumed. Stop if a supported backend rejects the frozen 255-byte form.
- **Report after:** the complete API migration plus focused conformance, before full QA if that is a natural checkpoint
- **Report to:** the operator, who will relay progress to the orchestrator

## Suggested Next Move

Read this handoff, `AGENTS.md`, contract 021, milestone g02.028, Card 225, and
the current credential implementations. Verify the base. Characterize every
slot call site before changing the public type, then land the value and
migration as one coherent source break.

## Completion Protocol

### Before you start

1. Confirm worktree and branch.
2. Fetch and confirm `HEAD == origin/main`.
3. Confirm `git merge-base --is-ancestor ae8852e725acefc96847579b4b2035113914fbbb HEAD` and handoff presence.
4. Read the named refs.
5. Run `effigy tasks`, `effigy doctor`, and `effigy test --plan`.

### While you work

- Execute only Card 225 in meaningful commits.
- Report coherent chunks with exact checks.
- Stop for contract, platform, or scope failure.

### When the assigned runway is complete

1. Run focused validation and `effigy qa`.
2. Update Card 225, g02.028, evidence log, inventories, and next-task state.
3. Push `codex/scoped-credentials-card225`.
4. Open a PR against current pushed `main`, linking all refs and evidence.
5. Report the PR URL. Do not merge.

### Review and merge path

The orchestrator reviews; self-review may be recorded as a PR comment.
Requested changes: none yet. Merge requires explicit operator authorisation.

- **Closeout refs:** contract 021; g02.028; Card 225; generation index; implementation log

### Handoff closeout

Keep every status and pointer honest; record blockers rather than weakening the contract.
