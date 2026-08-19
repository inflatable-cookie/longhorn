# Agent Control Adoption Kit

Date: 2026-08-19
Scope: g02.033 (Cards 235-237)

## What happened

The adoption kit for contract 022: a composition guide a consumer card
can execute blind, a canonical skill plus finder script, one-command
effigy install into a scratch git repo (never a sibling app), a qa
drift lock against `CONTROL_TOOL_NAMES`, and a skill-only dogfood pass
against the packaged proof app.

Worktree was the launcher-provided
`/Users/tom/.t3/worktrees/longhorn/t3code-72ca9155` on
`t3code/read-agent-control-handoff`. Planning base `982eff16` is an
ancestor; the handoff was in `HEAD`.

## Install seam

`effigy --repo <consumer> agent-control:install-skill` cannot work:
`--repo` switches catalogs, and it is still consumed after `--`. Extra
args reach Rhai tasks only. The selector is therefore
`effigy agent-control:install-skill -- <git-repo>`. Logged in
PAPERCUTS. Fixtures prove install, no-op re-run, non-repo refuse, and
no writes outside `.claude/skills/agent-control/`.

## Dogfood finding (authorised)

The first skill-only pass reached listen and stalled. The listen cursor
used the ring's `nextSeq` as `sinceSeq`; the shim filters
`seq > sinceSeq`, so the first event after subscribe is dropped. Card
234's two-client e2e hid it (two `console.log`s; the second event
cleared the cursor). Reported; operator authorised the fix. Cursor is
now the last delivered event seq. Conformance
`listen_delivers_the_first_event_after_subscribe` locks the
single-trigger path.

Skill updates from the run: ack vs `resources/updated`; no
`list_commands` tool — ids come from the operator or composition
(`proof:ping` is the worked-example smoke check).

Re-run:
`examples/agent-control-proof/evidence/2026-08-19T22-06-06-skill-dogfood/`
(`appHeldFocus: false`, token redacted, findings empty).

## Evidence

- Guide `docs/guides/agent-control-composition.md`, indexed
- Skill `skills/agent-control/SKILL.md` (`longhorn_version: "0.1.0"`)
- Finder `skills/agent-control/scripts/find-instance.ts`
- `check:agent-control-skill` in `qa` and `health`
- `agent-control:install-skill`
- Dogfood receipt and unfocused screenshot as above

## Worker loop

Handoff
`docs/handoffs/20260819-222034-agent-control-adoption-kit-worker.md`.
Launcher worktree used as supplied. Do not merge — operator only.
