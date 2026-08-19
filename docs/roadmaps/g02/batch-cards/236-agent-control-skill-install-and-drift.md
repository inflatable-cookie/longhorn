# 236 Agent Control Skill Install And Drift Lock

Status: done 2026-08-19
Owner: Longhorn maintainers
Roadmap: g02.033
Governing refs: contract 022; contracts 001, 003; effigy conventions
(`effigy.toml`, existing `check:*` selectors)
Depends on: Card 235
Auto-start next card: yes — Card 237 in the same lane

## Objective

Skill installation into a consumer repo is one effigy invocation, and
the skill cannot silently drift from the tool surface it documents.

## Scope

- **Install selector.** An effigy task (working name
  `agent-control:install-skill`) that copies `skills/agent-control/`
  into a target repo's `.claude/skills/agent-control/`, driven by the
  existing `--repo <PATH>` convention. Behavior: refuses a target that
  is not a git repo; overwrites a previously installed copy only when
  the installed version stamp is older or equal, and says what it did;
  never touches anything outside `.claude/skills/agent-control/`.
  Idempotent re-run is a no-op that says so. This is the one sanctioned
  cross-repo write and it is operator-invoked, not automatic —
  consistent with contract 003's migration-batch rule (installation
  happens when a consumer's own adoption card runs it).
- **Drift lock.** A `check:agent-control-skill` selector in `qa`:
  parses the skill's tool table and asserts it matches the core crate's
  wire vocabulary (the Card 229/233 conformance list is the authority —
  reuse its source of truth rather than a second hand-typed list), and
  asserts the skill's `longhorn_version` stamp matches the workspace
  version. A tool added to the server without a skill row, a skill row
  for a tool that does not exist, or a stale stamp each fail `qa` with
  a message naming the mismatch.
- **Doctor visibility.** If the repo's doctor scan surface admits it
  cheaply, surface the same check's result there; do not build new
  doctor machinery for it.

## Acceptance Criteria

- [x] install into a scratch git repo works, re-run is a stated no-op,
      non-repo target refused, nothing outside the skill dir touched —
      all fixture-proved
- [x] drift check fails on: a removed tool, an added tool, an edited
      tool name, a stale version stamp — each proved by a fixture that
      breaks it on purpose
- [x] the vocabulary source of truth is shared with the conformance
      suite, not retyped
- [x] `effigy qa` passes with the new check in the board
- [x] no consumer repo touched in this card

## Validation

`effigy qa`; the install fixtures against a temp repo; `effigy doctor`.

## Stop Conditions

- effigy's task surface cannot express the cross-repo copy under
  `--repo` without new runner capability — stop and report;
- the drift check cannot reach a single source of truth for the
  vocabulary without exporting new core-crate surface — report first,
  per the standing rule.

## Closeout

Status: done 2026-08-19, same branch and worktree as Card 235.

`CONTROL_TOOL_NAMES` in `longhorn-agent-control` is the shared
vocabulary; conformance and `check:agent-control-skill` both use it.
Install is `effigy agent-control:install-skill -- <git-repo>` — the
global `--repo` flag switches catalogs and cannot name the target
(PAPERCUTS, 2026-08-19). Doctor runs the same check via `health`.

## Continuation

Card 237 closes the lane: the skill proved by an agent actually
following it.
